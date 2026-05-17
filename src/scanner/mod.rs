use crate::db::{Database, FileEntry};
use chrono::{DateTime, Utc};
use log::{error, info};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use walkdir::WalkDir;

pub struct Scanner {
    db: Arc<Mutex<Database>>,
    extensions: Vec<String>,
}

impl Scanner {
    pub fn new(db: Arc<Mutex<Database>>, extensions: Vec<String>) -> Self {
        Self { db, extensions }
    }

    pub async fn scan_path(&self, path_id: i64, root_path: &str) -> Result<usize, String> {
        info!("Starting scan for path_id={}, root={}", path_id, root_path);
        let path = PathBuf::from(root_path);

        if !path.exists() {
            return Err(format!("Path does not exist: {}", root_path));
        }

        let mut count = 0;

        for entry in WalkDir::new(&path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();

            if !file_path.is_file() {
                continue;
            }

            if let Some(ext) = file_path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if !self.extensions.contains(&ext_lower) {
                    continue;
                }

                if let Some(file_entry) = self.create_file_entry(path_id, file_path) {
                    let db = self.db.lock().await;
                    if let Err(e) = db.upsert_file(&file_entry) {
                        error!("Failed to insert file: {:?}", e);
                    } else {
                        count += 1;
                    }
                }
            }
        }

        info!("Scan complete: {} files found", count);
        Ok(count)
    }

    fn create_file_entry(&self, path_id: i64, file_path: &Path) -> Option<FileEntry> {
        let metadata = file_path.metadata().ok()?;
        let modified_at: DateTime<Utc> = metadata.modified().ok()?.into();

        let file_name = file_path
            .file_name()
            ?.to_string_lossy()
            .to_string();

        let file_ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut file_entry = FileEntry {
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
        };

        // Generate thumbnail at frame 300
        if let Some(ffmpeg_path) = Self::get_ffmpeg() {
            let data_dir = crate::utils::get_data_dir();
            let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path);
            match gen.generate_thumbnail_at_frame(&file_entry.file_path, 300) {
                Ok(thumb_path) => {
                    file_entry.thumbnail_path = Some(thumb_path);
                }
                Err(e) => {
                    log::warn!("Failed to generate thumbnail for {}: {}", file_entry.file_path, e);
                }
            }
        }

        Some(file_entry)
    }

    fn get_ffmpeg() -> Option<std::path::PathBuf> {
        let data_dir = crate::utils::get_data_dir();
        let ffmpeg_dir = data_dir.join("ffmpeg");

        #[cfg(target_os = "windows")]
        {
            let path = ffmpeg_dir.join("ffmpeg.exe");
            if path.exists() { return Some(path); }
            if std::process::Command::new("where").arg("ffmpeg").output()
                .map(|o| o.status.success()).unwrap_or(false) {
                return Some(std::path::PathBuf::from("ffmpeg"));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let path = ffmpeg_dir.join("ffmpeg");
            if path.exists() { return Some(path); }
            return Some(std::path::PathBuf::from("ffmpeg"));
        }
        None
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