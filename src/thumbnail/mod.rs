use log::{error, info};
use md5;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use crate::db::Database;

pub struct ThumbnailGenerator {
    root_dir: PathBuf,
    ffmpeg_path: PathBuf,
    width: u32,
    height: u32,
    db: Arc<Mutex<Database>>,
}

impl ThumbnailGenerator {
    pub fn new(root_dir: &Path, ffmpeg_path: &Path, db: Arc<Mutex<Database>>) -> Self {
        Self {
            root_dir: root_dir.to_path_buf(),
            ffmpeg_path: ffmpeg_path.to_path_buf(),
            width: 384,
            height: 216,
            db,
        }
    }

    pub fn get_thumbs_dir(&self) -> PathBuf {
        self.root_dir.join("thumbs")
    }

    pub fn get_thumbnail_path(&self, video_path: &str, file_id: Option<i64>) -> PathBuf {
        if let Some(fid) = file_id {
            if let Ok(Some(filename)) = self.db.lock().unwrap().get_thumb_filename(fid) {
                if !filename.contains('/') && !filename.contains('\\') && !filename.contains("..") {
                    let path = self.get_thumbs_dir().join(&filename);
                    if path.exists() {
                        return path;
                    }
                } else {
                    log::error!("Invalid thumbnail filename from DB for file_id={}: {}", fid, filename);
                }
            }
        }

        let hash = format!("{:x}", md5::compute(video_path.as_bytes()));
        let md5_path = self.get_thumbs_dir().join(format!("{}.jpg", hash));

        if md5_path.exists() {
            if let Some(fid) = file_id {
                let uuid = uuid::Uuid::new_v4().to_string();
                let new_filename = format!("{}.jpg", uuid);
                let new_path = self.get_thumbs_dir().join(&new_filename);
                let db = self.db.lock().unwrap();
                if std::fs::rename(&md5_path, &new_path).is_ok() {
                    if let Err(e) = db.set_thumb_filename(fid, &new_filename) {
                        log::error!("Failed to save mapping for file_id={}: {}", fid, e);
                        let _ = std::fs::rename(&new_path, &md5_path);
                        return md5_path;
                    }
                    info!("Migrated MD5 thumbnail to UUID for file_id={}", fid);
                    return new_path;
                }
            }
            return md5_path;
        }

        md5_path
    }

    pub fn thumbnail_exists(&self, video_path: &str, file_id: Option<i64>) -> bool {
        self.get_thumbnail_path(video_path, file_id).exists()
    }

    pub fn generate_thumbnail(&self, video_path: &str, file_id: Option<i64>) -> Result<String, String> {
        let thumb_path = self.get_thumbnail_path(video_path, file_id);

        if thumb_path.exists() {
            info!("Thumbnail already exists: {:?}", thumb_path);
            return Ok(thumb_path.to_string_lossy().to_string());
        }

        std::fs::create_dir_all(self.get_thumbs_dir())
            .map_err(|e| format!("Failed to create thumbs directory: {}", e))?;

        info!("Generating thumbnail for: {}", video_path);

        let output = Command::new(&self.ffmpeg_path)
            .args(["-y", "-i", video_path])
            .args(["-ss", "00:00:01"])
            .args(["-vframes", "1"])
            .args(["-vf", &format!("scale={}:{}:force_original_aspect_ratio=decrease", self.width, self.height)])
            .args(["-q:v", "2"])
            .arg(&thumb_path)
            .output()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ffmpeg failed: {}", stderr));
        }

        let uuid = uuid::Uuid::new_v4().to_string();
        let new_filename = format!("{}.jpg", uuid);
        let final_path = self.get_thumbs_dir().join(&new_filename);
        if std::fs::rename(&thumb_path, &final_path).is_ok() {
            if let Some(fid) = file_id {
                let db = self.db.lock().unwrap();
                if let Err(e) = db.set_thumb_filename(fid, &new_filename) {
                    log::error!("Failed to save thumbnail mapping for file_id={}: {}", fid, e);
                    let _ = std::fs::remove_file(&final_path);
                    return Err(format!("Failed to save thumbnail mapping: {}", e));
                }
            }
            info!("Thumbnail generated: {:?}", final_path);
            return Ok(final_path.to_string_lossy().to_string());
        }

        info!("Thumbnail generated: {:?}", thumb_path);
        Ok(thumb_path.to_string_lossy().to_string())
    }

    pub fn generate_thumbnail_at_time(&self, video_path: &str, time_seconds: f64, file_id: Option<i64>) -> Result<String, String> {
        let thumb_path = self.get_thumbnail_path(video_path, file_id);

        std::fs::create_dir_all(self.get_thumbs_dir())
            .map_err(|e| format!("Failed to create thumbs directory: {}", e))?;

        let time_str = format!("{:02}:{:02}:{:02}",
            (time_seconds / 3600.0) as u32,
            ((time_seconds % 3600.0) / 60.0) as u32,
            (time_seconds % 60.0) as u32
        );

        info!("Generating thumbnail at {} for: {}", time_str, video_path);

        let output = Command::new(&self.ffmpeg_path)
            .args(["-y", "-ss", &time_str])
            .args(["-i", video_path])
            .args(["-vframes", "1"])
            .args(["-vf", &format!("scale={}:{}:force_original_aspect_ratio=decrease", self.width, self.height)])
            .args(["-q:v", "2"])
            .arg(&thumb_path)
            .output()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ffmpeg failed: {}", stderr));
        }

        let uuid = uuid::Uuid::new_v4().to_string();
        let new_filename = format!("{}.jpg", uuid);
        let final_path = self.get_thumbs_dir().join(&new_filename);
        if std::fs::rename(&thumb_path, &final_path).is_ok() {
            if let Some(fid) = file_id {
                let db = self.db.lock().unwrap();
                if let Err(e) = db.set_thumb_filename(fid, &new_filename) {
                    log::error!("Failed to save thumbnail mapping for file_id={}: {}", fid, e);
                    let _ = std::fs::remove_file(&final_path);
                    return Err(format!("Failed to save thumbnail mapping: {}", e));
                }
            }
            info!("Thumbnail generated: {:?}", final_path);
            return Ok(final_path.to_string_lossy().to_string());
        }

        info!("Thumbnail generated: {:?}", thumb_path);
        Ok(thumb_path.to_string_lossy().to_string())
    }

    fn get_frame_count(&self, video_path: &str) -> Result<u32, String> {
        let ffprobe_path = self.ffmpeg_path.parent()
            .map(|p| p.join(if cfg!(target_os = "windows") { "ffprobe.exe" } else { "ffprobe" }))
            .unwrap_or_else(|| PathBuf::from(if cfg!(target_os = "windows") { "ffprobe.exe" } else { "ffprobe" }));

        let output = Command::new(&ffprobe_path)
            .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=nb_frames", "-of", "default=nokey=1:noprint_wrappers=1"])
            .arg(video_path)
            .output();

        match output {
            Ok(out) => {
                let count_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !count_str.is_empty() && count_str != "N/A" {
                    count_str.parse::<u32>()
                        .map_err(|e| format!("Failed to parse frame count: {}", e))
                } else {
                    Err("Frame count not available in stream metadata".to_string())
                }
            }
            Err(e) => Err(format!("ffprobe failed: {}", e))
        }
    }

    pub fn generate_thumbnail_at_frame(&self, video_path: &str, target_frame: u32, file_id: Option<i64>) -> Result<String, String> {
        let thumb_path = self.get_thumbnail_path(video_path, file_id);

        if thumb_path.exists() {
            info!("Thumbnail already exists: {:?}", thumb_path);
            return Ok(thumb_path.to_string_lossy().to_string());
        }

        std::fs::create_dir_all(self.get_thumbs_dir())
            .map_err(|e| format!("Failed to create thumbs directory: {}", e))?;

        let frame_to_use = match self.get_frame_count(video_path) {
            Ok(count) => {
                if count >= target_frame {
                    target_frame
                } else {
                    log::warn!("Video has only {} frames, using frame 1 instead of {}", count, target_frame);
                    1
                }
            }
            Err(e) => {
                log::warn!("Could not get frame count ({}), trying frame {} directly", e, target_frame);
                target_frame
            }
        };

        info!("Generating thumbnail at frame {} for: {}", frame_to_use, video_path);

        let frame_idx = frame_to_use.saturating_sub(1);
        let output = Command::new(&self.ffmpeg_path)
            .args(["-y", "-i", video_path])
            .args(["-vf", &format!("select=eq(n\\,{}),scale={}:{}", frame_idx, self.width, self.height)])
            .args(["-vframes", "1"])
            .args(["-q:v", "2"])
            .arg(&thumb_path)
            .output()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if frame_to_use != 1 {
                log::warn!("Frame {} failed, trying frame 1: {}", frame_to_use, stderr);
                return self.generate_thumbnail_at_frame(video_path, 1, file_id);
            }
            return Err(format!("ffmpeg failed: {}", stderr));
        }

        let uuid = uuid::Uuid::new_v4().to_string();
        let new_filename = format!("{}.jpg", uuid);
        let final_path = self.get_thumbs_dir().join(&new_filename);
        if std::fs::rename(&thumb_path, &final_path).is_ok() {
            if let Some(fid) = file_id {
                let db = self.db.lock().unwrap();
                if let Err(e) = db.set_thumb_filename(fid, &new_filename) {
                    log::error!("Failed to save thumbnail mapping for file_id={}: {}", fid, e);
                    let _ = std::fs::remove_file(&final_path);
                    return Err(format!("Failed to save thumbnail mapping: {}", e));
                }
            }
            info!("Thumbnail generated: {:?}", final_path);
            return Ok(final_path.to_string_lossy().to_string());
        }

        info!("Thumbnail generated: {:?}", thumb_path);
        Ok(thumb_path.to_string_lossy().to_string())
    }

    pub fn play_video(&self, video_path: &str, player_path: &str) -> Result<(), String> {
        let mut cmd = if player_path.is_empty() {
            #[cfg(target_os = "windows")]
            {
                let mut c = Command::new("cmd");
                c.args(["/C", "start", "", video_path]);
                c
            }
            #[cfg(target_os = "macos")]
            {
                let mut c = Command::new("open");
                c.arg(video_path);
                c
            }
            #[cfg(target_os = "linux")]
            {
                let mut c = Command::new("xdg-open");
                c.arg(video_path);
                c
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            {
                return Err("No default video player configured for this platform. Please set a custom player in settings.".to_string());
            }
        } else {
            let mut c = Command::new(player_path);
            c.arg(video_path);
            c
        };

        cmd.spawn()
            .map_err(|e| format!("Failed to start player: {}", e))?;

        Ok(())
    }

    pub fn delete_thumbnail(&self, file_path: &str, file_id: Option<i64>) {
        if let Some(fid) = file_id {
            let db = self.db.lock().unwrap();
            if let Ok(Some(filename)) = db.get_thumb_filename(fid) {
                if !filename.contains('/') && !filename.contains('\\') && !filename.contains("..") {
                    let thumb_path = self.get_thumbs_dir().join(&filename);
                    if thumb_path.exists() {
                        if let Err(e) = std::fs::remove_file(&thumb_path) {
                            error!("Failed to delete thumbnail: {}", e);
                        } else {
                            info!("Deleted thumbnail: {:?}", thumb_path);
                        }
                    }
                } else {
                    log::error!("Invalid thumbnail filename in DB for file_id={}: {}", fid, filename);
                }
            }
            if let Err(e) = db.delete_thumb_mapping(fid) {
                log::error!("Failed to delete thumbnail mapping for file_id={}: {}", fid, e);
            }
        }

        let hash = format!("{:x}", md5::compute(file_path.as_bytes()));
        let md5_path = self.get_thumbs_dir().join(format!("{}.jpg", hash));
        if md5_path.exists() {
            if let Err(e) = std::fs::remove_file(&md5_path) {
                error!("Failed to delete MD5 thumbnail: {}", e);
            }
        }
    }
}
