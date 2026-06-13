use std::path::PathBuf;

/// Get the effective data directory from config or system default
pub fn get_data_dir() -> PathBuf {
    let config = crate::config::Config::load();
    let dir = config.get_data_dir();
    log::info!("get_data_dir() resolved to: {:?}", dir);
    dir
}

/// Find ffmpeg executable: first next to exe, then in data_dir/ffmpeg/, then in PATH
pub fn get_ffmpeg_path() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

    #[cfg(target_os = "windows")]
    {
        let candidates = [
            exe_dir.join("ffmpeg").join("ffmpeg.exe"),
            exe_dir.join("ffmpeg.exe"),
            get_data_dir().join("ffmpeg").join("ffmpeg.exe"),
        ];
        for path in &candidates {
            if path.exists() {
                return Some(path.clone());
            }
        }
        if let Ok(output) = std::process::Command::new("where").arg("ffmpeg").output() {
            if output.status.success() {
                return Some(PathBuf::from("ffmpeg"));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let candidates = [
            exe_dir.join("ffmpeg").join("ffmpeg"),
            exe_dir.join("ffmpeg"),
            get_data_dir().join("ffmpeg").join("ffmpeg"),
        ];
        for path in &candidates {
            if path.exists() {
                return Some(path.clone());
            }
        }
        return Some(PathBuf::from("ffmpeg"));
    }

    None
}

/// Find ffprobe executable: first next to exe, then in data_dir/ffmpeg/, then in PATH
pub fn get_ffprobe_path() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

    #[cfg(target_os = "windows")]
    {
        let candidates = [
            exe_dir.join("ffmpeg").join("ffprobe.exe"),
            exe_dir.join("ffprobe.exe"),
            get_data_dir().join("ffmpeg").join("ffprobe.exe"),
        ];
        for path in &candidates {
            if path.exists() {
                return Some(path.clone());
            }
        }
        if let Ok(output) = std::process::Command::new("where").arg("ffprobe").output() {
            if output.status.success() {
                return Some(PathBuf::from("ffprobe"));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let candidates = [
            exe_dir.join("ffmpeg").join("ffprobe"),
            exe_dir.join("ffprobe"),
            get_data_dir().join("ffmpeg").join("ffprobe"),
        ];
        for path in &candidates {
            if path.exists() {
                return Some(path.clone());
            }
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
