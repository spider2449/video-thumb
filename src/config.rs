use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Window size and position
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 720.0,
            x: None,
            y: None,
        }
    }
}

/// Application configuration stored in config.json (in exe directory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Custom data directory for thumbnails and database.
    /// If not set, defaults to system data_local_dir
    pub data_dir: Option<PathBuf>,
    /// Window size and position
    pub window: Option<WindowState>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: None,
            window: None,
        }
    }
}

impl Config {
    /// Get the path to config.json (in exe directory)
    fn config_path() -> PathBuf {
        let exe_path = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."));
        exe_path
            .parent()
            .map(|p| p.join("config.json"))
            .unwrap_or_else(|| PathBuf::from("config.json"))
    }

    /// Load config from config.json, or return default if not exists
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(config) => {
                            log::info!("Loaded config from {:?}", path);
                            return config;
                        }
                        Err(e) => {
                            log::warn!("Failed to parse config.json: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read config.json: {}", e);
                }
            }
        }
        Self::default()
    }

    /// Save config to config.json
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write config.json: {}", e))?;
        log::info!("Saved config to {:?}", path);
        Ok(())
    }

    /// Get the effective data directory
    /// Returns either the configured custom path or a "cache" directory next to the executable
    pub fn get_data_dir(&self) -> PathBuf {
        if let Some(ref custom_dir) = self.data_dir {
            custom_dir.clone()
        } else {
            let exe_path = std::env::current_exe()
                .unwrap_or_else(|_| PathBuf::from("."));
            exe_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("cache")
        }
    }

    /// Set custom data directory
    pub fn set_data_dir(&mut self, path: PathBuf) {
        self.data_dir = Some(path);
    }

    /// Clear custom data directory (use default)
    pub fn clear_data_dir(&mut self) {
        self.data_dir = None;
    }

    /// Get window state, or return default
    pub fn get_window_state(&self) -> WindowState {
        self.window.unwrap_or_default()
    }

    /// Set window state
    pub fn set_window_state(&mut self, state: WindowState) {
        self.window = Some(state);
    }
}
