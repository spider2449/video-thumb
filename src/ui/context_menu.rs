use super::icons::Icon;
use crate::app::App;
use crate::db::{FileEntry, SafeMutexExt};
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ContextMenu {
    pub file: Option<FileEntry>,
    pub click_pos: egui::Pos2,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self {
            file: None,
            click_pos: egui::Pos2::ZERO,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) {
        let file = match self.file.clone() {
            Some(f) => f,
            None => return,
        };

        let area_response = egui::Area::new(egui::Id::new("context_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(self.click_pos)
            .show(ctx, |ui| {
                let width = (ctx.screen_rect().width() - 32.0).clamp(0.0, 250.0);
                egui::Frame::popup(ui.style())
                    .inner_margin(10)
                    .corner_radius(10)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.visuals_mut().widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;

                        egui::ScrollArea::vertical()
                            .max_height((ctx.screen_rect().height() - 28.0).max(0.0))
                            .show(ui, |ui| self.show_items(ui, app, &file));
                    });
            });

        if ctx.input(|i| i.pointer.primary_clicked())
            && ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|pos| !area_response.response.rect.contains(pos))
        {
            self.file = None;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.file = None;
        }
    }

    fn show_items(&mut self, ui: &mut egui::Ui, app: &mut App, file: &FileEntry) {
        let file_id = file.id;
        let lang = app.lang;
        ui.add(egui::Label::new(egui::RichText::new(&file.file_name).strong()).truncate())
            .on_hover_text(&file.file_path);
        ui.separator();

        if Self::action(ui, Icon::Play, crate::lang::t(lang, "play")).clicked() {
            self.play_file_internal(&file.file_path, app);
            self.file = None;
        }

        ui.separator();

        ui.label(
            egui::RichText::new(crate::lang::t(lang, "rating_label"))
                .small()
                .weak(),
        );
        let current_rating = file.rating;
        ui.horizontal(|ui| {
            for i in 1..=5 {
                let star_symbol = if i <= current_rating { "★" } else { "☆" };
                let mut star = egui::RichText::new(star_symbol).size(18.0);
                if i <= current_rating {
                    star = star.color(ui.visuals().warn_fg_color);
                }
                if ui
                    .add_sized([28.0, 26.0], egui::Button::new(star))
                    .on_hover_text(i.to_string())
                    .clicked()
                {
                    self.apply_rating(file_id, i, app);
                }
            }
        });

        ui.separator();

        ui.label(
            egui::RichText::new(crate::lang::t(lang, "tags_label"))
                .small()
                .weak(),
        );
        let file_tags = app
            .db
            .safe_lock()
            .get_tags_for_file(file_id)
            .unwrap_or_default();

        for tag in &file_tags {
            let color = crate::utils::hex_to_color(&tag.color);
            let tag_id = tag.id;
            let file_id_copy = file_id;
            ui.horizontal(|ui| {
                if ui.add(Icon::Close.button(ui)).on_hover_text(crate::lang::t(lang, "remove_tag")).clicked() {
                    app.db
                        .safe_lock()
                        .remove_tag_from_file(file_id_copy, tag_id)
                        .ok();
                    app.file_tag_map
                        .entry(file_id_copy)
                        .and_modify(|tags| tags.retain(|&t| t != tag_id));
                    app.invalidate_cache();
                }
                ui.add(egui::Label::new(egui::RichText::new(&tag.name).color(color)).truncate())
                    .on_hover_text(&tag.name);
            });
        }

        let available_tags: Vec<_> = app
            .tags
            .iter()
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
                            if ui
                                .selectable_label(
                                    false,
                                    egui::RichText::new(&tag_name)
                                        .color(crate::utils::hex_to_color(&tag_color)),
                                )
                                .clicked()
                            {
                                app.db.safe_lock().add_tag_to_file(file_id, tag_id).ok();
                                app.file_tag_map.entry(file_id).or_default().push(tag_id);
                                app.invalidate_cache();
                            }
                        }
                    });
            });
        }

        if Self::action(ui, Icon::TagPlus, crate::lang::t(lang, "manage_tags")).clicked() {
            app.show_tag_manager = true;
            self.file = None;
        }

        ui.separator();

        if Self::action(ui, Icon::Camera, crate::lang::t(lang, "screenshot")).clicked() {
            app.screenshot_dialog = Some(crate::ui::ScreenshotDialog::new(file.clone(), ui.ctx()));
            self.file = None;
        }

        ui.separator();

        let other_tabs: Vec<(i64, String, String)> = app
            .watched_paths
            .iter()
            .filter(|p| p.id != file.path_id)
            .map(|p| (p.id, p.path.clone(), p.display_name.clone()))
            .collect();

        if !other_tabs.is_empty() {
            ui.collapsing(crate::lang::t(lang, "move_to_tab"), |ui| {
                for (dest_id, dest_path, dest_name) in &other_tabs {
                    if Self::action(ui, Icon::Folder, dest_name.as_str()).clicked() {
                        let new_path = std::path::PathBuf::from(dest_path).join(&file.file_name);
                        let new_path_str = new_path.to_string_lossy().to_string();
                        self.move_file_to_tab_internal(file, *dest_id, &new_path_str, app);
                    }
                }
            });
        } else {
            ui.label(crate::lang::t(lang, "no_other_tabs"));
        }

        ui.separator();

        if Self::action(ui, Icon::Folder, crate::lang::t(lang, "open_folder")).clicked() {
            self.open_file_location_internal(file);
            self.file = None;
        }

        if Self::action(ui, Icon::Copy, crate::lang::t(lang, "copy_path")).clicked() {
            ui.ctx().copy_text(file.file_path.clone());
            self.file = None;
        }

        ui.separator();

        if file.has_custom_thumb && Self::action(ui, Icon::Refresh, crate::lang::t(lang, "reset_thumb")).clicked()
        {
            self.reset_thumbnail_internal(file, app);
        }

        let remove = egui::RichText::new(crate::lang::t(lang, "remove_record"))
            .color(ui.visuals().error_fg_color);
        if Self::action(ui, Icon::Trash, remove).clicked() {
            self.remove_file_internal(file.clone(), app);
        }
    }

    fn action(ui: &mut egui::Ui, icon: Icon, text: impl Into<egui::WidgetText>) -> egui::Response {
        ui.add(
            icon.labeled(ui, text)
                .truncate()
                .min_size(egui::vec2(ui.available_width(), 26.0))
                .corner_radius(5),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn play_file_internal(&self, file_path: &str, app: &App) {
        app.play_video_file(file_path);
    }

    fn apply_rating(&mut self, file_id: i64, rating: i32, app: &mut App) {
        app.db.safe_lock()
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

    fn reset_thumbnail_internal(&self, file: &FileEntry, app: &mut App) {
        if let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() {
            let root_dir = crate::utils::get_data_dir();
            let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path, app.db.clone());

            generator.delete_thumbnail(&file.file_path, Some(file.id));

            if let Ok(thumb_path) = generator.generate_thumbnail(&file.file_path, Some(file.id)) {
                if app.db.safe_lock()
                    .clear_custom_thumb(file.id)
                    .is_ok() {
                    for files in app.files.values_mut() {
                        let mut vec = (**files).clone();
                        for f in vec.iter_mut() {
                            if f.id == file.id {
                                f.thumbnail_path = Some(thumb_path.clone());
                                f.has_custom_thumb = false;
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
        app.db.safe_lock().delete_file(file.id).ok();

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

        app.filter_cache = None;
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
                app.db.safe_lock()
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
