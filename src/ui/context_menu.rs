use crate::app::App;
use crate::db::FileEntry;
use eframe::egui;
use std::path::PathBuf;

pub struct ContextMenu {
    pub file: Option<FileEntry>,
    pub add_tag_name: String,
    pub time_input: String,
    pub click_pos: egui::Pos2,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self {
            file: None,
            add_tag_name: String::new(),
            time_input: String::new(),
            click_pos: egui::Pos2::ZERO,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) {
        let file = match self.file.clone() {
            Some(f) => f,
            None => return,
        };

        let file_id = file.id;
        let file_path = file.file_path.clone();
        let file_for_location = file.clone();
        let file_for_remove = file.clone();
        let file_for_capture = file.clone();

        let area_response = egui::Area::new(egui::Id::new("context_menu"))
            .fixed_pos(self.click_pos)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        ui.set_min_width(200.0);

                        if ui.button("▶ 播放").clicked() {
                            self.play_file_internal(&file_path, app);
                            self.file = None;
                        }

                        ui.separator();

                        ui.label("評分:");
                        let current_rating = file.rating;
                        ui.horizontal(|ui| {
                            for i in 1..=5 {
                                let star_symbol = if i <= current_rating { "★" } else { "☆" };
                                let rating_val = i as i32;
                                if ui.button(star_symbol).clicked() {
                                    self.apply_rating(file_id, rating_val, app);
                                }
                            }
                        });

                        ui.separator();

                        ui.label("標籤:");
                        let file_tags = app.db.blocking_lock()
                            .get_tags_for_file(file_id)
                            .unwrap_or_default();

                        for tag in &file_tags {
                            let color = self.hex_to_color(&tag.color);
                            let tag_id = tag.id;
                            let file_id_copy = file_id;
                            ui.horizontal(|ui| {
                                ui.colored_label(color, &tag.name);
                                if ui.button("×").clicked() {
                                    app.db.blocking_lock()
                                        .remove_tag_from_file(file_id_copy, tag_id)
                                        .ok();
                                }
                            });
                        }

                        // ComboBox to select existing tags
                        let available_tags: Vec<_> = app.tags.iter()
                            .filter(|t| !file_tags.iter().any(|ft| ft.id == t.id))
                            .collect();

                        if !available_tags.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("新增:");
                                egui::ComboBox::from_id_salt("add_tag_combo")
                                    .width(120.0)
                                    .selected_text("選擇標籤")
                                    .show_ui(ui, |ui| {
                                        for tag in available_tags {
                                            let color = self.hex_to_color(&tag.color);
                                            if ui.selectable_label(false, egui::RichText::new(&tag.name).color(color)).clicked() {
                                                app.db.blocking_lock()
                                                    .add_tag_to_file(file_id, tag.id)
                                                    .ok();
                                            }
                                        }
                                    });
                            });
                        }

                        if ui.button("🏷️ 管理標籤...").clicked() {
                            app.show_tag_manager = true;
                            self.file = None;
                        }

                        ui.separator();

                        if ui.button("📷 截圖...").clicked() {
                            app.screenshot_dialog = Some(crate::ui::ScreenshotDialog::new(file_for_capture));
                            self.file = None; // Close context menu
                        }


                        ui.separator();

                        if ui.button("📂 開啟所在資料夾").clicked() {
                            self.open_file_location_internal(&file_for_location);
                            self.file = None;
                        }

                        let clipboard_path = file.file_path.clone();
                        if ui.button("📋 複製路徑").clicked() {
                            ctx.copy_text(clipboard_path);
                            self.file = None;
                        }

                        ui.separator();

                        if file.has_custom_thumb {
                            if ui.button("🔄 恢復預設縮圖").clicked() {
                                self.reset_thumbnail_internal(&file, app);
                            }
                        }

                        if ui.button("❌ 移除記錄").clicked() {
                            self.remove_file_internal(file_for_remove, app);
                        }
                    });
            });

        // Close on left-click outside the menu area
        if ctx.input(|i| i.pointer.primary_clicked()) {
            if let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !area_response.response.rect.contains(pointer_pos) {
                    self.file = None;
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.file = None;
        }
    }

    fn play_file_internal(&self, file_path: &str, app: &App) {
        let Some(ffmpeg_path) = get_ffmpeg_path() else {
            log::error!("ffmpeg not found");
            return;
        };

        let player = app.db.blocking_lock()
            .get_setting("default_player")
            .ok()
            .flatten()
            .unwrap_or_default();

        let root_dir = get_data_dir();
        let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path);
        generator.play_video(file_path, &player).ok();
    }

    fn apply_rating(&mut self, file_id: i64, rating: i32, app: &mut App) {
        app.db.blocking_lock()
            .update_file_rating(file_id, rating)
            .ok();
        app.files.values_mut()
            .for_each(|files| {
                if let Some(f) = files.iter_mut().find(|f| f.id == file_id) {
                    f.rating = rating;
                }
            });
        if let Some(f) = self.file.as_mut() {
            if f.id == file_id {
                f.rating = rating;
            }
        }
    }

    fn add_tag_internal(&mut self, file_id: i64, app: &mut App) {
        let tag_name = self.add_tag_name.trim();
        if tag_name.is_empty() {
            return;
        }

        let db = app.db.blocking_lock();

        let tag_id = if let Ok(tags) = db.get_all_tags() {
            if let Some(tag) = tags.iter().find(|t| t.name == tag_name) {
                tag.id
            } else {
                db.create_tag(tag_name, "#808080").unwrap_or(0)
            }
        } else {
            return;
        };

        db.add_tag_to_file(file_id, tag_id).ok();
        app.tags = db.get_all_tags().unwrap_or_default();
        self.add_tag_name.clear();
    }

    fn capture_screenshot_internal(&self, file: &FileEntry, app: &App) {
        let Some(ffmpeg_path) = get_ffmpeg_path() else {
            log::error!("ffmpeg not found");
            return;
        };

        let time_seconds: f64 = if self.time_input.contains(':') {
            parse_time_string(&self.time_input).unwrap_or(1.0)
        } else {
            self.time_input.parse().unwrap_or(1.0)
        };

        let root_dir = get_data_dir();
        let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path);

        if let Ok(thumb_path) = generator.generate_thumbnail_at_time(&file.file_path, time_seconds) {
            app.db.blocking_lock()
                .update_thumbnail(file.id, &thumb_path)
                .ok();
        }
    }

    fn reset_thumbnail_internal(&self, file: &FileEntry, app: &mut App) {
        if let Some(ffmpeg_path) = get_ffmpeg_path() {
            let root_dir = get_data_dir();
            let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path);

            generator.delete_thumbnail(&file.file_path);

            if let Ok(thumb_path) = generator.generate_thumbnail(&file.file_path) {
                app.db.blocking_lock()
                    .update_thumbnail(file.id, &thumb_path)
                    .ok();
            }
        }
    }

    fn open_file_location_internal(&self, file: &FileEntry) {
        let path = PathBuf::from(&file.file_path);
        if let Some(parent) = path.parent() {
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("explorer")
                    .arg(parent)
                    .spawn().ok();
            }
            #[cfg(target_os = "macos")]
            {
                std::process::Command::new("open")
                    .arg(parent)
                    .spawn().ok();
            }
            #[cfg(target_os = "linux")]
            {
                std::process::Command::new("xdg-open")
                    .arg(parent)
                    .spawn().ok();
            }
        }
    }

    fn remove_file_internal(&mut self, file: FileEntry, app: &mut App) {
        app.db.blocking_lock().delete_file(file.id).ok();

        app.files.values_mut()
            .for_each(|files| {
                files.retain(|f| f.id != file.id);
            });

        if let Some(ffmpeg_path) = get_ffmpeg_path() {
            let root_dir = get_data_dir();
            let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path);
            generator.delete_thumbnail(&file.file_path);
        }

        self.file = None;
    }

    fn hex_to_color(&self, hex: &str) -> egui::Color32 {
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
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self::new()
    }
}

fn get_ffmpeg_path() -> Option<PathBuf> {
    let root_dir = get_data_dir();
    let ffmpeg_dir = root_dir.join("ffmpeg");

    #[cfg(target_os = "windows")]
    {
        let path = ffmpeg_dir.join("ffmpeg.exe");
        if path.exists() { return Some(path); }

        std::process::Command::new("where")
            .arg("ffmpeg")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|_| PathBuf::from("ffmpeg"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let path = ffmpeg_dir.join("ffmpeg");
        if path.exists() { return Some(path); }
        Some(PathBuf::from("ffmpeg"))
    }
}

fn get_data_dir() -> std::path::PathBuf {
    crate::app::get_data_dir()
}

fn parse_time_string(time: &str) -> Option<f64> {
    let parts: Vec<&str> = time.split(':').collect();
    match parts.len() {
        1 => parts[0].parse().ok(),
        2 => {
            let min: f64 = parts[0].parse().ok()?;
            let sec: f64 = parts[1].parse().ok()?;
            Some(min * 60.0 + sec)
        }
        3 => {
            let hr: f64 = parts[0].parse().ok()?;
            let min: f64 = parts[1].parse().ok()?;
            let sec: f64 = parts[2].parse().ok()?;
            Some(hr * 3600.0 + min * 60.0 + sec)
        }
        _ => None,
    }
}
