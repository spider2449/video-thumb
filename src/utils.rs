use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use crate::config::Config;

static CONFIG_CACHE: OnceLock<RwLock<Config>> = OnceLock::new();

pub fn init_config() {
    let config = Config::load();
    CONFIG_CACHE.set(RwLock::new(config)).ok();
}

pub fn reload_config() {
    if let Some(cache) = CONFIG_CACHE.get() {
        let config = Config::load();
        if let Ok(mut guard) = cache.write() {
            *guard = config;
        }
    }
}

pub fn get_config() -> Config {
    if let Some(cache) = CONFIG_CACHE.get() {
        if let Ok(guard) = cache.read() {
            return guard.clone();
        }
    }
    Config::default()
}

pub fn get_data_dir() -> PathBuf {
    if let Some(cache) = CONFIG_CACHE.get() {
        if let Ok(guard) = cache.read() {
            return guard.get_data_dir();
        }
    }
    Config::default().get_data_dir()
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
    let parse = |slice: Option<&str>| slice.and_then(|s| u8::from_str_radix(s, 16).ok());
    match (
        hex.is_ascii(),
        parse(hex.get(0..2)),
        parse(hex.get(2..4)),
        parse(hex.get(4..6)),
    ) {
        (true, Some(r), Some(g), Some(b)) => egui::Color32::from_rgb(r, g, b),
        _ => egui::Color32::GRAY,
    }
}
