use crate::app::App;
use crate::config::Config;
use eframe::egui;

pub struct SettingsDialog {
    pub player_path: String,
    pub thumb_size: String,
    pub data_dir: String,
    pub error: Option<String>,
    pub close_after_save: bool,
}

impl SettingsDialog {
    pub fn new() -> Self {
        Self {
            player_path: String::new(),
            thumb_size: "384".to_string(),
            data_dir: String::new(),
            error: None,
            close_after_save: false,
        }
    }

    pub fn load(&mut self, app: &App) {
        let db = app.db.lock().unwrap();
        self.player_path = db.get_setting("default_player")
            .ok()
            .flatten()
            .unwrap_or_default();
        self.thumb_size = db.get_setting("thumb_size")
            .ok()
            .flatten()
            .unwrap_or_else(|| "384".to_string());
        // Load data_dir from config.json
        let config = Config::load();
        self.data_dir = config.data_dir
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.close_after_save = false;
    }

    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) -> bool {
        egui::Window::new("設定")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(450.0);
            ui.set_min_height(200.0);

            ui.horizontal(|ui| {
                ui.label("播放器路徑:");
                ui.text_edit_singleline(&mut self.player_path);
                if ui.button("選擇...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("執行檔", &["exe", "app", ""])
                        .pick_file()
                    {
                        self.player_path = path.to_string_lossy().to_string();
                    }
                }
            });
            ui.label("留空使用系統預設播放器");

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("縮圖大小:");
                ui.text_edit_singleline(&mut self.thumb_size);
                ui.label("像素");
            });

            ui.add_space(8.0);

            // Data directory setting
            ui.horizontal(|ui| {
                ui.label("快取目錄:");
                ui.text_edit_singleline(&mut self.data_dir);
                if ui.button("選擇...").clicked() {
                    if let Some(selected_path) = rfd::FileDialog::new().pick_folder() {
                        self.data_dir = selected_path.to_string_lossy().to_string();
                    }
                }
            });
            ui.label("留空使用系統預設目錄 (需重新啟動生效)");

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.horizontal(|ui| {
                if ui.button("關閉").clicked() {
                    self.close_after_save = true;
                }
                if ui.button("儲存").clicked() {
                    if let Err(e) = self.save(app) {
                        self.error = Some(e);
                    } else {
                        self.close_after_save = true;
                    }
                }
            });
        });

        if self.close_after_save {
            self.reset();
            return false;
        }
        true
    }

    fn save(&self, app: &mut App) -> Result<(), String> {
        let db = app.db.lock().unwrap();

        db.set_setting("default_player", &self.player_path)
            .map_err(|e| format!("儲存播放器路徑失敗: {}", e))?;

        let size: u32 = self.thumb_size.parse()
            .map_err(|_| "請輸入有效數字".to_string())?;

        if size < 64 || size > 1024 {
            return Err("縮圖大小必須在 64-1024 之間".to_string());
        }

        db.set_setting("thumb_size", &self.thumb_size)
            .map_err(|e| format!("儲存縮圖大小失敗: {}", e))?;

        app.thumbnail_size = size as f32;

        // Save data_dir to config.json
        let mut config = Config::load();
        if self.data_dir.is_empty() {
            config.clear_data_dir();
        } else {
            config.set_data_dir(std::path::PathBuf::from(&self.data_dir));
        }
        config.save().map_err(|e| format!("儲存快取目錄失敗: {}", e))?;

        Ok(())
    }

    pub fn reset(&mut self) {
        self.player_path.clear();
        self.thumb_size = "384".to_string();
        self.data_dir.clear();
        self.error = None;
        self.close_after_save = false;
    }
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self::new()
    }
}
