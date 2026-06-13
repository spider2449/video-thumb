use crate::db::{Database, FileEntry, SafeMutexExt};
use chrono::{DateTime, Utc};
use log::{error, info};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

pub struct Scanner {
    db: Arc<Mutex<Database>>,
    extensions: Vec<String>,
    cancel_flag: Arc<AtomicBool>,
}

/// Result of a scan operation
pub struct ScanFileResult {
    pub count: usize,
    pub files_needing_thumbnails: Vec<String>,
}

impl Scanner {
    pub fn new(db: Arc<Mutex<Database>>, extensions: Vec<String>, cancel_flag: Arc<AtomicBool>) -> Self {
        Self { db, extensions, cancel_flag }
    }

    pub async fn scan_path(&self, path_id: i64, root_path: &str) -> Result<ScanFileResult, String> {
        info!("Starting scan for path_id={}, root={}", path_id, root_path);
        let path = PathBuf::from(root_path);

        if !path.exists() {
            return Err(format!("Path does not exist: {}", root_path));
        }

        let extensions = self.extensions.clone();
        let root_str = root_path.to_string();
        let cancel_flag = self.cancel_flag.clone();

        let files = tokio::task::spawn_blocking(move || {
            Self::blocking_scan(&root_str, path_id, &extensions, &cancel_flag)
        })
        .await
        .map_err(|e| format!("task join error: {e}"))?;

        // Insert files into database first (so we have file_ids for thumbnail lookup)
        let mut count = 0;
        let mut files_needing_thumbnails = Vec::new();

        let db_files;
        {
            let db = self.db.safe_lock();
            if let Err(e) = db.begin_transaction() {
                error!("Failed to begin transaction: {:?}", e);
            }

            if let Err(e) = db.upsert_files_batch(&files) {
                error!("Failed to batch insert files: {:?}", e);
            } else {
                count = files.len();
            }

            let found_paths: HashSet<String> = files.iter()
                .map(|f| f.file_path.clone())
                .collect();
            if let Ok(existing_files) = db.get_files_for_path(path_id) {
                let mut deleted = 0;
                for existing in &existing_files {
                    if !found_paths.contains(&existing.file_path) {
                        if let Err(e) = db.delete_file(existing.id) {
                            error!("Failed to delete stale record for {}: {:?}", existing.file_path, e);
                        } else {
                            deleted += 1;
                        }
                    }
                }
                if deleted > 0 {
                    info!("Deleted {} stale records for files no longer on disk", deleted);
                }
            }

            if let Err(e) = db.commit_transaction() {
                error!("Failed to commit transaction: {:?}", e);
            }

            db_files = db.get_files_for_path(path_id).unwrap_or_default();
        }

        // Now check thumbnail existence using file_ids from DB (lock released)
        let data_dir = crate::utils::get_data_dir();
        let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &PathBuf::from(""), self.db.clone());

        let path_to_id: std::collections::HashMap<String, i64> = db_files.iter()
            .map(|f| (f.file_path.clone(), f.id))
            .collect();
        for file_entry in &files {
            let file_id = path_to_id.get(&file_entry.file_path).copied();
            let thumb_path = gen.get_thumbnail_path(&file_entry.file_path, file_id);
            if !thumb_path.exists() {
                files_needing_thumbnails.push(file_entry.file_path.clone());
            }
        }

        info!("Scan complete: {} files found, {} need thumbnails", count, files_needing_thumbnails.len());
        Ok(ScanFileResult { count, files_needing_thumbnails })
    }

    fn blocking_scan(path: &str, path_id: i64, extensions: &[String], cancel_flag: &Arc<AtomicBool>) -> Vec<FileEntry> {
        let root = PathBuf::from(path);
        let mut entries = Vec::new();

        for (i, entry) in WalkDir::new(&root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .enumerate()
        {
            if i % 100 == 0 && cancel_flag.load(Ordering::Relaxed) {
                info!("Scan cancelled");
                return entries;
            }

            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }

            if let Some(ext) = file_path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if !extensions.contains(&ext_lower) {
                    continue;
                }

                if let Some(file_entry) = Self::create_file_entry(path_id, file_path) {
                    entries.push(file_entry);
                }
            }
        }

        entries
    }

    fn create_file_entry(path_id: i64, file_path: &Path) -> Option<FileEntry> {
        let metadata = file_path.metadata().ok()?;
        let modified_at: DateTime<Utc> = metadata.modified().ok()?.into();

        let file_name = file_path
            .file_name()?
            .to_string_lossy()
            .to_string();

        let file_ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        Some(FileEntry {
            id: 0,
            path_id,
            file_path: file_path.to_string_lossy().to_string(),
            file_name,
            file_ext,
            thumbnail_path: None,
            file_size: metadata.len() as i64,
            modified_at: modified_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            rating: 0,
            has_custom_thumb: false,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }
}

pub fn get_file_duration(video_path: &str, ffmpeg_path: &Path) -> Option<f64> {
    let output = std::process::Command::new(ffmpeg_path)
        .args(["-i", video_path])
        .args(["-hide_banner"])
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let duration_line = stderr.lines().find(|l| l.contains("Duration:"))?;

    let duration_str = duration_line
        .split("Duration:")
        .nth(1)?
        .split(',')
        .next()?
        .trim();

    let parts: Vec<&str> = duration_str.split(':').collect();
    if parts.len() < 3 {
        return None;
    }

    let hours: f64 = parts[0].parse().ok()?;
    let minutes: f64 = parts[1].parse().ok()?;
    let seconds: f64 = parts[2].parse().ok()?;

    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}
