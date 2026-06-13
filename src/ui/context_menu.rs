use crate::app::App;
use crate::db::FileEntry;
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;

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
        let lang = app.lang;

        let area_response = egui::Area::new(egui::Id::new("context_menu"))
            .fixed_pos(self.click_pos)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        ui.set_min_width(200.0);

                        if ui.button(crate::lang::t(lang, "play")).clicked() {
                            self.play_file_internal(&file_path, app);
                            self.file = None;
                        }

                        ui.separator();

                        ui.label(crate::lang::t(lang, "rating_label"));
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

                        ui.label(crate::lang::t(lang, "tags_label"));
                        let file_tags = app.db.lock().unwrap()
                            .get_tags_for_file(file_id)
                            .unwrap_or_default();

                        for tag in &file_tags {
                            let color = crate::utils::hex_to_color(&tag.color);
                            let tag_id = tag.id;
                            let file_id_copy = file_id;
                            ui.horizontal(|ui| {
                                ui.colored_label(color, &tag.name);
                                if ui.button("×").clicked() {
                                    app.db.lock().unwrap()
                                        .remove_tag_from_file(file_id_copy, tag_id)
                                        .ok();
                                    app.file_tag_map.entry(file_id_copy)
                                        .and_modify(|tags| tags.retain(|&t| t != tag_id));
                                    app.invalidate_cache();
                                }
                            });
                        }

                        let available_tags: Vec<_> = app.tags.iter()
                            .filter(|t| !file_tags.iter().any(|ft| ft.id == t.id))
                            .map(|t| (t.id, t.name.clone(), t.color.clone()))
                            .collect();

                        if !available_tags.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label(crate::lang::t(lang, "add_tag"));
                                egui::ComboBox::from_id_salt("add_tag_combo")
                                    .width(120.0)
                                    .selected_text(crate::lang::t(lang, "select_tag"))
                                    .show_ui(ui, |ui| {
                                        for (tag_id, tag_name, tag_color) in available_tags {
                                            if ui.selectable_label(false, egui::RichText::new(&tag_name).color(crate::utils::hex_to_color(&tag_color))).clicked() {
                                                app.db.lock().unwrap()
                                                    .add_tag_to_file(file_id, tag_id)
                                                    .ok();
                                                app.file_tag_map.entry(file_id)
                                                    .or_default()
                                                    .push(tag_id);
                                                app.invalidate_cache();
                                            }
                                        }
                                    });
                            });
                        }

                        if ui.button(crate::lang::t(lang, "manage_tags")).clicked() {
                            app.show_tag_manager = true;
                            self.file = None;
                        }

                        ui.separator();

                        if ui.button(crate::lang::t(lang, "screenshot")).clicked() {
                            app.screenshot_dialog = Some(crate::ui::ScreenshotDialog::new(file_for_capture));
                            self.file = None;
                        }

                        ui.separator();

                        let other_tabs: Vec<(i64, String, String)> = app.watched_paths.iter()
                            .filter(|p| p.id != file.path_id)
                            .map(|p| (p.id, p.path.clone(), p.display_name.clone()))
                            .collect();

                        if !other_tabs.is_empty() {
                            ui.collapsing(crate::lang::t(lang, "move_to_tab"), |ui| {
                                for (dest_id, dest_path, dest_name) in &other_tabs {
                                    if ui.button(dest_name).clicked() {
                                        let new_path = std::path::PathBuf::from(dest_path)
                                            .join(&file.file_name);
                                        let new_path_str = new_path.to_string_lossy().to_string();
                                        self.move_file_to_tab_internal(&file, *dest_id, &new_path_str, app);
                                    }
                                }
                            });
                        } else {
                            ui.label(crate::lang::t(lang, "no_other_tabs"));
                        }

                        ui.separator();

                        if ui.button(crate::lang::t(lang, "open_folder")).clicked() {
                            self.open_file_location_internal(&file_for_location);
                            self.file = None;
                        }

                        let clipboard_path = file.file_path.clone();
                        if ui.button(crate::lang::t(lang, "copy_path")).clicked() {
                            ctx.copy_text(clipboard_path);
                            self.file = None;
                        }

                        ui.separator();

                        if file.has_custom_thumb {
                            if ui.button(crate::lang::t(lang, "reset_thumb")).clicked() {
                                self.reset_thumbnail_internal(&file, app);
                            }
                        }

                        if ui.button(crate::lang::t(lang, "remove_record")).clicked() {
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
        app.play_video_file(file_path);
    }

    fn apply_rating(&mut self, file_id: i64, rating: i32, app: &mut App) {
        app.db.lock().unwrap()
            .update_file_rating(file_id, rating)
            .ok();
        app.files.values_mut()
            .for_each(|files| {
                let mut vec = (**files).clone();
                if let Some(f) = vec.iter_mut().find(|f| f.id == file_id) {
                    f.rating = rating;
                }
                *files = Arc::new(vec);
            });
        if let Some(f) = self.file.as_mut() {
            if f.id == file_id {
                f.rating = rating;
            }
        }
        app.invalidate_cache();
    }

    fn add_tag_internal(&mut self, file_id: i64, app: &mut App) {
        let tag_name = self.add_tag_name.trim();
        if tag_name.is_empty() {
            return;
        }

        let db = app.db.lock().unwrap();

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
        let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() else {
            log::error!("ffmpeg not found");
            return;
        };

        let time_seconds: f64 = if self.time_input.contains(':') {
            parse_time_string(&self.time_input).unwrap_or(1.0)
        } else {
            self.time_input.parse().unwrap_or(1.0)
        };

        let root_dir = crate::utils::get_data_dir();
        let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path, app.db.clone());

        if let Ok(thumb_path) = generator.generate_thumbnail_at_time(&file.file_path, time_seconds, Some(file.id)) {
            app.db.lock().unwrap()
                .update_thumbnail(file.id, &thumb_path)
                .ok();
        }
    }

    fn reset_thumbnail_internal(&self, file: &FileEntry, app: &mut App) {
        if let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() {
            let root_dir = crate::utils::get_data_dir();
            let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path, app.db.clone());

            generator.delete_thumbnail(&file.file_path, Some(file.id));

            if let Ok(thumb_path) = generator.generate_thumbnail(&file.file_path, Some(file.id)) {
                if app.db.lock().unwrap()
                    .update_thumbnail(file.id, &thumb_path)
                    .is_ok() {
                    for files in app.files.values_mut() {
                        let mut vec = (**files).clone();
                        for f in vec.iter_mut() {
                            if f.id == file.id {
                                f.thumbnail_path = Some(thumb_path.clone());
                                f.has_custom_thumb = true;
                            }
                        }
                        *files = Arc::new(vec);
                    }
                    app.invalidate_cache();
                }
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
        app.db.lock().unwrap().delete_file(file.id).ok();

        app.files.values_mut()
            .for_each(|files| {
                let mut vec = (**files).clone();
                vec.retain(|f| f.id != file.id);
                *files = Arc::new(vec);
            });

        // Clean up tag map
        app.file_tag_map.remove(&file.id);

        if let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() {
            let root_dir = crate::utils::get_data_dir();
            let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path, app.db.clone());
            generator.delete_thumbnail(&file.file_path, Some(file.id));
        }

        self.file = None;
    }

    fn move_file_to_tab_internal(&mut self, file: &FileEntry, new_path_id: i64, new_file_path: &str, app: &mut App) {
        let source_path = std::path::PathBuf::from(&file.file_path);
        let dest_path = std::path::PathBuf::from(new_file_path);

        // Try direct rename first (same drive)
        let move_result = std::fs::rename(&source_path, &dest_path);

        // If cross-drive, copy then delete
        let move_result = move_result.or_else(|_| {
            std::fs::copy(&source_path, &dest_path)
                .and_then(|_| std::fs::remove_file(&source_path))
        });

        match move_result {
            Ok(_) => {
                // Update database
                app.db.lock().unwrap()
                    .move_file(file.id, new_path_id, new_file_path)
                    .ok();

                // Clean up moved file's tag map entry
                app.file_tag_map.remove(&file.id);

                // Invalidate cache to trigger reload with correct data
                app.invalidate_cache();

                self.file = None;
            }
            Err(e) => {
                log::error!("Failed to move file: {}", e);
            }
        }
    }
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self::new()
    }
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
