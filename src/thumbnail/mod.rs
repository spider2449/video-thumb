use log::{error, info};
use md5;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct ThumbnailGenerator {
    root_dir: PathBuf,
    ffmpeg_path: PathBuf,
    width: u32,
    height: u32,
}

impl ThumbnailGenerator {
    pub fn new(root_dir: &Path, ffmpeg_path: &Path) -> Self {
        Self {
            root_dir: root_dir.to_path_buf(),
            ffmpeg_path: ffmpeg_path.to_path_buf(),
            width: 384,
            height: 216,
        }
    }

    pub fn get_thumbs_dir(&self) -> PathBuf {
        self.root_dir.join("thumbs")
    }

    pub fn get_thumbnail_path(&self, video_path: &str) -> PathBuf {
        let hash = format!("{:x}", md5::compute(video_path.as_bytes()));
        self.get_thumbs_dir().join(format!("{}.jpg", hash))
    }

    pub fn thumbnail_exists(&self, video_path: &str) -> bool {
        self.get_thumbnail_path(video_path).exists()
    }

    pub fn generate_thumbnail(&self, video_path: &str) -> Result<String, String> {
        let thumb_path = self.get_thumbnail_path(video_path);

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
            .args(["-vf", &format!("scale={}:{}", self.width, self.height)])
            .args(["-q:v", "2"])
            .arg(&thumb_path)
            .output()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ffmpeg failed: {}", stderr));
        }

        info!("Thumbnail generated: {:?}", thumb_path);
        Ok(thumb_path.to_string_lossy().to_string())
    }

    pub fn generate_thumbnail_at_time(&self, video_path: &str, time_seconds: f64) -> Result<String, String> {
        let thumb_path = self.get_thumbnail_path(video_path);

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
            .args(["-vf", &format!("scale={}:{}", self.width, self.height)])
            .args(["-q:v", "2"])
            .arg(&thumb_path)
            .output()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ffmpeg failed: {}", stderr));
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
        } else {
            let mut c = Command::new(player_path);
            c.arg(video_path);
            c
        };

        cmd.spawn()
            .map_err(|e| format!("Failed to start player: {}", e))?;

        Ok(())
    }

    pub fn delete_thumbnail(&self, file_path: &str) {
        let thumb_path = self.get_thumbnail_path(file_path);
        if thumb_path.exists() {
            if let Err(e) = std::fs::remove_file(&thumb_path) {
                error!("Failed to delete thumbnail: {}", e);
            } else {
                info!("Deleted thumbnail: {:?}", thumb_path);
            }
        }
    }
}