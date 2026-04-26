use crate::app::App;
use eframe::egui;

pub struct AddPathDialog {
    pub path: String,
    pub display_name: String,
    pub error: Option<String>,
    pub should_close: bool,
}

impl AddPathDialog {
    pub fn new() -> Self {
        Self {
            path: String::new(),
            display_name: String::new(),
            error: None,
            should_close: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) -> bool {
        if self.should_close {
            return false;
        }

        egui::Window::new("新增監控路徑")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(400.0);

                ui.horizontal(|ui| {
                    ui.label("資料夾路徑:");
                    ui.text_edit_singleline(&mut self.path);
                    if ui.button("選擇...").clicked() {
                        if let Some(selected_path) = rfd::FileDialog::new()
                            .pick_folder()
                        {
                            self.path = selected_path.to_string_lossy().to_string();
                            if self.display_name.is_empty() {
                                self.display_name = selected_path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                            }
                        }
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("顯示名稱:");
                    ui.text_edit_singleline(&mut self.display_name);
                });

                if let Some(error) = &self.error {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::RED, error);
                }

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.should_close = true;
                        app.show_add_path = false;
                    }
                    if ui.button("新增").clicked() {
                        self.add_path(app);
                    }
                });
            });

        !self.should_close
    }

    fn add_path(&mut self, app: &mut App) {
        if self.path.is_empty() {
            self.error = Some("請選擇資料夾路徑".to_string());
            return;
        }
        if !std::path::Path::new(&self.path).exists() {
            self.error = Some("路徑不存在".to_string());
            return;
        }
        if !std::path::Path::new(&self.path).is_dir() {
            self.error = Some("路徑不是有效的資料夾".to_string());
            return;
        }

        let path = self.path.clone();
        let display_name = if self.display_name.is_empty() {
            std::path::Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone())
        } else {
            self.display_name.clone()
        };

        let db = app.db.blocking_lock();
        match db.path_exists(&path) {
            Ok(true) => {
                self.error = Some("此路徑已存在".to_string());
            }
            Ok(false) => {
                match db.add_path(&path, &display_name) {
                    Ok(id) => {
                        app.watched_paths.push(crate::db::WatchedPath {
                            id,
                            path,
                            display_name,
                            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        });
                        app.current_path_id = Some(id);
                        app.show_add_path = false;
                        self.should_close = true;
                    }
                    Err(e) => {
                        self.error = Some(format!("新增失敗: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("查詢失敗: {}", e));
            }
        }
    }

    pub fn reset(&mut self) {
        self.path.clear();
        self.display_name.clear();
        self.error = None;
        self.should_close = false;
    }
}

impl Default for AddPathDialog {
    fn default() -> Self {
        Self::new()
    }
}