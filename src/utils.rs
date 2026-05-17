use std::path::PathBuf;

/// Get the effective data directory from config or system default
pub fn get_data_dir() -> PathBuf {
    let config = crate::config::Config::load();
    let dir = config.get_data_dir();
    log::info!("get_data_dir() resolved to: {:?}", dir);
    dir
}

/// Find ffmpeg executable: first in data_dir/ffmpeg/, then in PATH
pub fn get_ffmpeg_path() -> Option<PathBuf> {
    let ffmpeg_dir = get_data_dir().join("ffmpeg");

    #[cfg(target_os = "windows")]
    {
        let path = ffmpeg_dir.join("ffmpeg.exe");
        if path.exists() {
            return Some(path);
        }
        if std::process::Command::new("where")
            .arg("ffmpeg")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .is_some()
        {
            return Some(PathBuf::from("ffmpeg"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let path = ffmpeg_dir.join("ffmpeg");
        if path.exists() {
            return Some(path);
        }
        return Some(PathBuf::from("ffmpeg"));
    }

    None
}

/// Find ffprobe executable: first in data_dir/ffmpeg/, then in PATH
pub fn get_ffprobe_path() -> Option<PathBuf> {
    let ffmpeg_dir = get_data_dir().join("ffmpeg");

    #[cfg(target_os = "windows")]
    {
        let path = ffmpeg_dir.join("ffprobe.exe");
        if path.exists() {
            return Some(path);
        }
        if std::process::Command::new("where")
            .arg("ffprobe")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .is_some()
        {
            return Some(PathBuf::from("ffprobe"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let path = ffmpeg_dir.join("ffprobe");
        if path.exists() {
            return Some(path);
        }
        return Some(PathBuf::from("ffprobe"));
    }

    None
}

/// Parse a hex color string (with or without #) into egui::Color32
pub fn hex_to_color(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
        egui::Color32::from_rgb(r, g, b)
    } else {
        egui::Color32::GRAY
    }
}
