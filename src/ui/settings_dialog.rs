use crate::app::App;
use eframe::egui;

pub struct SettingsDialog {
    pub player_path: String,
    pub thumb_size: String,
    pub error: Option<String>,
    pub close_after_save: bool,
}

impl SettingsDialog {
    pub fn new() -> Self {
        Self {
            player_path: String::new(),
            thumb_size: "384".to_string(),
            error: None,
            close_after_save: false,
        }
    }

    pub fn load(&mut self, app: &App) {
        let db = app.db.blocking_lock();
        self.player_path = db.get_setting("default_player")
            .ok()
            .flatten()
            .unwrap_or_default();
        self.thumb_size = db.get_setting("thumb_size")
            .ok()
            .flatten()
            .unwrap_or_else(|| "384".to_string());
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
        let db = app.db.blocking_lock();

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

        Ok(())
    }

    pub fn reset(&mut self) {
        self.player_path.clear();
        self.thumb_size = "384".to_string();
        self.error = None;
        self.close_after_save = false;
    }
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self::new()
    }
}