use crate::app::App;
use crate::config::Config;
use crate::lang::Lang;
use eframe::egui;

pub struct SettingsDialog {
    pub player_path: String,
    pub thumb_size: String,
    pub data_dir: String,
    pub error: Option<String>,
    pub close_after_save: bool,
    pub selected_lang: Lang,
}

impl SettingsDialog {
    pub fn new() -> Self {
        Self {
            player_path: String::new(),
            thumb_size: "384".to_string(),
            data_dir: String::new(),
            error: None,
            close_after_save: false,
            selected_lang: Lang::ZhTw,
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
        self.selected_lang = Lang::from_setting(
            &db.get_setting("language").ok().flatten().unwrap_or_else(|| "zh_tw".to_string())
        );
        let config = Config::load();
        self.data_dir = config.data_dir
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.close_after_save = false;
    }

    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) -> bool {
        let lang = app.lang;
        egui::Window::new(crate::lang::t(lang, "settings_title"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(450.0);
            ui.set_min_height(200.0);

            ui.horizontal(|ui| {
                ui.label(crate::lang::t(lang, "language"));
                for l in Lang::all() {
                    if ui.selectable_label(self.selected_lang == *l, l.display_name()).clicked() {
                        self.selected_lang = *l;
                    }
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(crate::lang::t(lang, "player_path"));
                ui.text_edit_singleline(&mut self.player_path);
                if ui.button(crate::lang::t(lang, "browse")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(crate::lang::t(lang, "executable"), &["exe", "app", ""])
                        .pick_file()
                    {
                        self.player_path = path.to_string_lossy().to_string();
                    }
                }
            });
            ui.label(crate::lang::t(lang, "player_hint"));

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(crate::lang::t(lang, "thumb_size_label"));
                ui.text_edit_singleline(&mut self.thumb_size);
                ui.label(crate::lang::t(lang, "pixels"));
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(crate::lang::t(lang, "cache_dir"));
                ui.text_edit_singleline(&mut self.data_dir);
                if ui.button(crate::lang::t(lang, "browse")).clicked() {
                    if let Some(selected_path) = rfd::FileDialog::new().pick_folder() {
                        self.data_dir = selected_path.to_string_lossy().to_string();
                    }
                }
            });
            ui.label(crate::lang::t(lang, "cache_dir_hint"));

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.horizontal(|ui| {
                if ui.button(crate::lang::t(lang, "close")).clicked() {
                    self.close_after_save = true;
                }
                if ui.button(crate::lang::t(lang, "save")).clicked() {
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
        let lang = app.lang;
        let db = app.db.lock().unwrap();

        db.set_setting("default_player", &self.player_path)
            .map_err(|e| format!("{}{}", crate::lang::t(lang, "save_player_failed_prefix"), e))?;

        let size: u32 = self.thumb_size.parse()
            .map_err(|_| crate::lang::t(lang, "invalid_number").to_string())?;

        if size < 64 || size > 1024 {
            return Err(crate::lang::t(lang, "thumb_size_range").to_string());
        }

        db.set_setting("thumb_size", &self.thumb_size)
            .map_err(|e| format!("{}{}", crate::lang::t(lang, "save_thumb_size_failed_prefix"), e))?;

        db.set_setting("language", self.selected_lang.as_setting())
            .map_err(|e| format!("Failed to save language: {}", e))?;

        app.thumbnail_size = size as f32;
        drop(db);

        app.lang = self.selected_lang;

        let mut config = Config::load();
        if self.data_dir.is_empty() {
            config.clear_data_dir();
        } else {
            config.set_data_dir(std::path::PathBuf::from(&self.data_dir));
        }
        config.save().map_err(|e| format!("{}{}", crate::lang::t(lang, "save_cache_dir_failed_prefix"), e))?;

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
