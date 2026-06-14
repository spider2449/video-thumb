use crate::app::SortBy;
use crate::db::SafeMutexExt;
use crate::App;
use crate::scanner::Scanner;
use eframe::egui;
use log::info;
use std::sync::atomic::Ordering;

pub struct MainWindow {}

impl MainWindow {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show_main(ctx: &egui::Context, app: &mut App) {
        let _available_width = ctx.input(|i| i.screen_rect.width()) - 40.0;
        let _cols = ((_available_width / app.thumbnail_size) as usize).max(1);
        let lang = app.lang;

        // Toolbar
        egui::TopBottomPanel::top("toolbar")
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(crate::lang::t(lang, "add_path")).clicked() {
                        app.show_add_path = true;
                        app.reset_add_path_dialog();
                    }

                    let scan_text = if app.scanning {
                        format!("{}{:.0}{}", crate::lang::t(lang, "scanning_prefix"), app.scan_progress * 100.0, crate::lang::t(lang, "scanning_suffix"))
                    } else {
                        crate::lang::t(lang, "update_db").to_string()
                    };

                    ui.add_enabled_ui(!app.scanning, |ui| {
                        if ui.button(&scan_text).clicked() {
                            Self::start_scan(app);
                        }
                    });

                    if ui.button(crate::lang::t(lang, "rescan_thumbs")).clicked() {
                        Self::clean_unused_thumbnails(app);
                    }

                    ui.separator();

                    ui.menu_button(crate::lang::t(lang, "sort"), |ui| {
                        if ui.selectable_label(app.sort_by == SortBy::Name, crate::lang::t(lang, "name")).clicked() {
                            app.sort_by = SortBy::Name;
                            app.invalidate_cache();
                            ui.close_menu();
                        }
                        if ui.selectable_label(app.sort_by == SortBy::Date, crate::lang::t(lang, "date")).clicked() {
                            app.sort_by = SortBy::Date;
                            app.invalidate_cache();
                            ui.close_menu();
                        }
                        if ui.selectable_label(app.sort_by == SortBy::Rating, crate::lang::t(lang, "rating")).clicked() {
                            app.sort_by = SortBy::Rating;
                            app.invalidate_cache();
                            ui.close_menu();
                        }
                    });

                    ui.separator();

                    ui.label(crate::lang::t(lang, "search"));
                    let prev_query = app.search_query.clone();
                    ui.text_edit_singleline(&mut app.search_query);
                    if app.search_query != prev_query {
                        app.search_last_changed = std::time::Instant::now();
                        app.search_debounce_pending = true;
                    }
                    if app.search_debounce_pending && app.search_last_changed.elapsed().as_millis() >= 200 {
                        app.invalidate_cache();
                        app.search_debounce_pending = false;
                    }

                    ui.separator();

                    ui.menu_button(crate::lang::t(lang, "rating_filter"), |ui| {
                        if ui.selectable_label(app.rating_filter.is_none(), crate::lang::t(lang, "all")).clicked() {
                            app.rating_filter = None;
                            app.invalidate_cache();
                            ui.close_menu();
                        }
                        for i in 1..=5 {
                            let label = format!("{}{}", i, crate::lang::t(lang, "stars_suffix"));
                            if ui.selectable_label(app.rating_filter == Some(i), &label).clicked() {
                                app.rating_filter = Some(i);
                                app.invalidate_cache();
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();

                    if !app.tags.is_empty() {
                        ui.menu_button(crate::lang::t(lang, "tag_filter"), |ui| {
                            if ui.selectable_label(app.tag_filter.is_none(), crate::lang::t(lang, "all")).clicked() {
                                app.tag_filter = None;
                                app.invalidate_cache();
                                ui.close_menu();
                            }
                            let tags_snapshot: Vec<_> = app.tags.clone();
                            for tag in &tags_snapshot {
                                let color = crate::utils::hex_to_color(&tag.color);
                                let is_selected = app.tag_filter == Some(tag.id);
                                if ui.selectable_label(is_selected, egui::RichText::new(&tag.name).color(color)).clicked() {
                                    app.tag_filter = Some(tag.id);
                                    app.invalidate_cache();
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.separator();
                    }

                    if ui.button(crate::lang::t(lang, "tag_manager")).clicked() {
                        app.show_tag_manager = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Slider::new(&mut app.thumbnail_size, 80.0..=400.0).text(crate::lang::t(lang, "size")));
                        ui.separator();

                        let theme_icon = if app.current_theme == "light" { "☽" } else { "☀" };
                        if ui.button(theme_icon).clicked() {
                            app.current_theme = if app.current_theme == "light" {
                                "dark".to_string()
                            } else {
                                "light".to_string()
                            };
                            if app.current_theme == "light" {
                                ctx.set_visuals(egui::Visuals::light());
                            } else {
                                ctx.set_visuals(egui::Visuals::dark());
                            }
                            let db = app.db.safe_lock();
                            let _ = db.set_setting("theme", &app.current_theme);
                        }

                        if ui.button(crate::lang::t(lang, "settings")).clicked() {
                            app.show_settings = true;
                        }
                    });
                });
            });

        // Tab bar
        egui::TopBottomPanel::top("tab_bar")
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    egui::ScrollArea::vertical()
                        .drag_to_scroll(false)
                        .show(ui, |ui| {
                            let tabs: Vec<(i64, String, bool)> = app.watched_paths.iter().map(|p| {
                                (p.id, p.display_name.clone(), app.current_path_id == Some(p.id))
                            }).collect();
                            for &(ref path_id, ref display_name, selected) in &tabs {
                                let button = egui::Button::new(display_name.as_str()).selected(selected);

                                if ui.add(button).clicked() {
                                    app.current_path_id = Some(*path_id);
                                    app.selected_file_id = None;
                                    app.clear_texture_cache();
                                    if !app.files.contains_key(path_id) {
                                        app.load_files_for_path(*path_id);
                                    }
                                }

                                if selected {
                                    if ui.button("×").clicked() {
                                        app.remove_path_confirm = Some((*path_id, display_name.clone()));
                                    }
                                }
                            }
                        });
                    ui.separator();
                    if ui.button(crate::lang::t(lang, "add_path")).clicked() {
                        app.show_add_path = true;
                        app.reset_add_path_dialog();
                    }
                });
            });

        // Main content
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let files = app.get_current_files();
                let scroll_target = app.scroll_to_idx.take();

                let available_width = ctx.input(|i| i.screen_rect.width()) - 40.0;
                let item_width = app.thumbnail_size;
                let item_height = app.thumbnail_size + 50.0;
                let items_per_row = (available_width / item_width).max(1.0) as usize;
                let row_height = item_height + 4.0;
                let total_rows = (files.len() + items_per_row - 1) / items_per_row;

                let scroll_area = egui::ScrollArea::vertical()
                    .id_salt(egui::Id::new("file_scroll_area"))
                    .auto_shrink([false; 2]);

                let scroll_output = scroll_area.show(ui, |ui| {
                    let clip_rect = ui.clip_rect();
                    let cursor_min_y = ui.cursor().min.y;

                    let visible_top = (clip_rect.min.y - cursor_min_y).max(0.0);
                    let visible_bottom = (clip_rect.max.y - cursor_min_y).max(0.0);

                    let first_visible_row = (visible_top / row_height).floor() as usize;
                    let last_visible_row = ((visible_bottom / row_height).ceil() as usize).min(total_rows);

                    let extra_rows = 2;
                    let render_first = first_visible_row.saturating_sub(extra_rows);
                    let render_last = (last_visible_row + extra_rows).min(total_rows);

                    if render_first > 0 {
                        ui.add_space(render_first as f32 * row_height);
                    }

                    for row in render_first..render_last {
                        ui.horizontal(|ui| {
                            let start_idx = row * items_per_row;
                            let end_idx = (start_idx + items_per_row).min(files.len());
                            for idx in start_idx..end_idx {
                                let file = &files[idx];
                                if scroll_target == Some(idx) {
                                    ui.scroll_to_cursor(Some(egui::Align::TOP));
                                }
                                Self::show_thumbnail_item(ui, file, app.thumbnail_size, ctx, app);
                            }
                        });
                    }

                    if render_last < total_rows {
                        ui.add_space((total_rows - render_last) as f32 * row_height);
                    }
                });

                let _ = scroll_output;
            });

    // Context menu overlay (rendered on top)
    {
        if let Some(mut menu) = app.context_menu.take() {
            menu.show(ctx, app);
            // Keep menu open if it still has a file, otherwise close it
            if menu.file.is_some() {
                app.context_menu = Some(menu);
            }
        }
    }

    // Screenshot dialog
    if let Some(mut dialog) = app.screenshot_dialog.take() {
        let keep_open = dialog.show(ctx, app);
        if keep_open {
            app.screenshot_dialog = Some(dialog);
        }
    }

    // Remove path confirmation dialog
    if let Some((path_id, display_name)) = app.remove_path_confirm.take() {
        let mut keep_open = true;
        egui::Window::new(crate::lang::t(lang, "confirm_remove_path"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(300.0);
                ui.add_space(8.0);
                ui.label(format!("{}{}{}", crate::lang::t(lang, "confirm_remove_prefix"), display_name, crate::lang::t(lang, "confirm_remove_suffix")));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(crate::lang::t(lang, "remove_only_records")).small().color(egui::Color32::GRAY));
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(crate::lang::t(lang, "confirm_remove")).clicked() {
                            let mut db = app.db.safe_lock();
                            let should_remove = db.remove_path(path_id).is_ok();
                            drop(db);
                            if should_remove {
                                app.watched_paths.retain(|p| p.id != path_id);
                                app.files.remove(&path_id);
                                app.current_path_id = app.watched_paths.first().map(|p| p.id);
                                if let Some(new_id) = app.current_path_id {
                                    app.load_files_for_path(new_id);
                                }
                            }
                            keep_open = false;
                        }
                        if ui.button(crate::lang::t(lang, "cancel")).clicked() {
                            keep_open = false;
                        }
                    });
                });
            });
        if keep_open {
            app.remove_path_confirm = Some((path_id, display_name));
        }
    }

    // Status bar
        egui::TopBottomPanel::bottom("status_bar")
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let file_count = app.current_path_id
                        .and_then(|id| app.files.get(&id))
                        .map(|files| files.len())
                        .unwrap_or(0);
                    ui.label(format!("{}{}{}", crate::lang::t(lang, "file_count_prefix"), file_count, crate::lang::t(lang, "file_count_suffix")));
                    if app.scanning {
                        ui.label(format!(" | {}", app.scan_status));
                    }
                    if let Some(path_id) = app.current_path_id {
                        if let Some(path) = app.watched_paths.iter().find(|p| p.id == path_id) {
                            ui.label(format!(" | {}", path.path));
                        }
                    }
                });
            });

        // Add Path Dialog
        if app.show_add_path {
            egui::Window::new(crate::lang::t(lang, "add_path_title"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_width(400.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(crate::lang::t(lang, "folder_path"));
                        ui.text_edit_singleline(&mut app.add_path_path);
                        if ui.button(crate::lang::t(lang, "browse")).clicked() {
                            if let Some(selected_path) = rfd::FileDialog::new().pick_folder() {
                                app.add_path_path = selected_path.to_string_lossy().to_string();
                                if app.add_path_display_name.is_empty() {
                                    app.add_path_display_name = selected_path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                }
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(crate::lang::t(lang, "display_name"));
                        ui.text_edit_singleline(&mut app.add_path_display_name);
                    });

                    if let Some(error) = &app.add_path_error {
                        ui.add_space(8.0);
                        ui.colored_label(egui::Color32::RED, error);
                    }

                    ui.add_space(16.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(crate::lang::t(lang, "cancel")).clicked() {
                            app.show_add_path = false;
                            app.reset_add_path_dialog();
                        }
                        if ui.button(crate::lang::t(lang, "add")).clicked() {
                            app.add_path();
                        }
                    });
                });
        }

        // Settings Dialog
    if app.show_settings {
        // Create dialog instance if does not exist
        if app.settings_dialog.is_none() {
            let mut dialog = crate::ui::SettingsDialog::new();
            dialog.load(app);
            app.settings_dialog = Some(dialog);
        }

        // Show the stored dialog (using take() to avoid borrow conflict)
        if let Some(mut dialog) = app.settings_dialog.take() {
            let keep_open = dialog.show(ctx, app);
            if keep_open {
                app.settings_dialog = Some(dialog);
            } else {
                app.show_settings = false;
            }
        }
    }

        // Tag Manager Dialog
    if app.show_tag_manager {
        // Create dialog instance if does not exist
        if app.tag_manager.is_none() {
            app.tag_manager = Some(crate::ui::TagManager::new());
        }

        // Show the stored dialog (using take() to avoid borrow conflict)
        if let Some(mut dialog) = app.tag_manager.take() {
            let keep_open = dialog.show(ctx, app);
            if keep_open {
                app.tag_manager = Some(dialog);
            } else {
                app.show_tag_manager = false;
            }
        }
    }
}

    fn show_thumbnail_item(ui: &mut egui::Ui, file: &crate::db::FileEntry, size: f32, ctx: &egui::Context, app: &mut App) {
        ui.allocate_ui([size, size + 50.0].into(), |ui| {
            ui.vertical(|ui| {
                let thumb_size_inner = size - 4.0;
                let thumb_rect = ui.allocate_rect(
                    egui::Rect::from_min_size(ui.cursor().min, [thumb_size_inner, thumb_size_inner].into()),
                    egui::Sense::click_and_drag(),
                );

                ui.painter().rect_filled(
                    thumb_rect.rect.shrink(1.0),
                    0.0,
                    egui::Color32::from_gray(30),
                );

                let is_selected = app.selected_file_id == Some(file.id);
                if is_selected {
                    ui.painter().rect_stroke(
                        thumb_rect.rect.shrink(1.0),
                        0.0,
                        egui::Stroke::new(3.0, ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Inside,
                    );
                }

                if let Some(thumb_path) = &file.thumbnail_path {
                    let path = std::path::PathBuf::from(thumb_path);
                    if path.exists() {
                        if let Some(tex) = Self::load_tex(ctx, app, &path) {
                            let tex_size = tex.size_vec2();
                            let container_size = thumb_rect.rect.size();

                            let aspect_tex = tex_size.x / tex_size.y;
                            let aspect_container = container_size.x / container_size.y;

                            let (draw_w, draw_h) = if aspect_tex > aspect_container {
                                let h = container_size.x / aspect_tex;
                                (container_size.x, h)
                            } else {
                                let w = container_size.y * aspect_tex;
                                (w, container_size.y)
                            };

                            let offset_x = (container_size.x - draw_w) / 2.0;
                            let offset_y = (container_size.y - draw_h) / 2.0;
                            let img_min = thumb_rect.rect.min + egui::Vec2::new(offset_x, offset_y);
                            let img_rect = egui::Rect::from_min_size(img_min, egui::Vec2::new(draw_w, draw_h));

                            ui.painter().image(tex.id(), img_rect, egui::Rect::from_min_max([0.0, 0.0].into(), [1.0, 1.0].into()), egui::Color32::WHITE);
                        }
                    } else {
                        ui.painter().rect_filled(thumb_rect.rect.shrink(1.0), 0.0, egui::Color32::from_gray(50));
                        ui.painter().text(thumb_rect.rect.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(24.0), egui::Color32::WHITE);
                    }
                } else {
                    ui.painter().rect_filled(thumb_rect.rect.shrink(1.0), 0.0, egui::Color32::from_gray(50));
                    ui.painter().text(thumb_rect.rect.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(24.0), egui::Color32::DARK_GRAY);
                }

                if file.has_custom_thumb {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(thumb_rect.rect.min, [8.0, 8.0].into()),
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(255, 200, 0, 200),
                    );
                }

                if thumb_rect.double_clicked() {
                    app.play_video_file(&file.file_path);
                }

                if thumb_rect.clicked() && !thumb_rect.double_clicked() {
                    app.selected_file_id = Some(file.id);
                }

                if thumb_rect.secondary_clicked() {
                    if app.context_menu.is_none() {
                        let mut menu = crate::ui::ContextMenu::new();
                        menu.file = Some(file.clone());
                        menu.click_pos = thumb_rect.interact_pointer_pos()
                            .unwrap_or_else(|| thumb_rect.rect.min);
                        app.context_menu = Some(menu);
                    } else {
                        if let Some(ref mut menu) = app.context_menu {
                            menu.file = Some(file.clone());
                            menu.click_pos = thumb_rect.interact_pointer_pos()
                                .unwrap_or_else(|| thumb_rect.rect.min);
                        }
                    }
                }

                ui.label(egui::RichText::new(&file.file_path).size(10.0).color(egui::Color32::LIGHT_GRAY));

                ui.horizontal_wrapped(|ui| {
                    for star in 1..=5 {
                        let color = if star <= file.rating { egui::Color32::from_rgb(255, 200, 0) } else { egui::Color32::from_gray(80) };
                        ui.label(egui::RichText::new(if star <= file.rating { "★" } else { "☆" }).color(color).size(14.0));
                    }
                });
            });
        });
    }

    fn load_tex(ctx: &egui::Context, app: &mut App, path: &std::path::PathBuf) -> Option<egui::TextureHandle> {
        let key = path.to_string_lossy().to_string();
        if let Some(h) = app.texture_cache.get(&key) {
            return Some(h.clone());
        }
        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let colors: Vec<egui::Color32> = rgba.pixels().map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])).collect();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color_img = egui::ColorImage { size, pixels: colors };
        let h = ctx.load_texture(&key, color_img, egui::TextureOptions::LINEAR);
        app.texture_cache.put(key, h.clone());
        Some(h)
    }

    fn clean_unused_thumbnails(app: &mut App) {
        let data_dir = crate::utils::get_data_dir();
        let thumbs_dir = data_dir.join("thumbs");

        if !thumbs_dir.exists() {
            let lang = app.lang;
            app.scan_status = crate::lang::t(lang, "thumbs_dir_not_exist").to_string();
            return;
        }

        let db = app.db.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        app.clean_thumbs_rx = Some(rx);
        app.scanning = true;
        app.scan_status = "Cleaning thumbnails...".to_string();

        std::thread::spawn(move || {
            let db_guard = db.safe_lock();

            let mut referenced: std::collections::HashSet<String> = db_guard
                .get_all_thumb_filenames()
                .unwrap_or_default()
                .into_iter()
                .collect();

            for legacy in db_guard.get_all_legacy_thumb_filenames().unwrap_or_default() {
                referenced.insert(legacy);
            }

            let orphan_mappings = db_guard.delete_orphan_thumb_mappings().unwrap_or(0);
            drop(db_guard);

            let mut deleted = 0usize;
            let entries = match std::fs::read_dir(&thumbs_dir) {
                Ok(e) => e,
                Err(e) => {
                    let _ = tx.send(format!("Cannot read thumbs dir: {}", e));
                    return;
                }
            };

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let filename = entry.file_name().to_string_lossy().to_string();
                if !referenced.contains(&filename) {
                    if std::fs::remove_file(entry.path()).is_ok() {
                        deleted += 1;
                    }
                }
            }

            let msg = format!("Deleted {} unused thumbnails, cleaned {} orphan records", deleted, orphan_mappings);
            let _ = tx.send(msg);
        });

        if let Some(ctx) = &app.egui_ctx {
            ctx.request_repaint();
        }
    }

    fn start_scan(app: &mut App) {
        if app.watched_paths.is_empty() { return; }

        let path_id = app.current_path_id
            .or_else(|| app.watched_paths.first().map(|p| p.id))
            .unwrap();
        let path = app.watched_paths.iter()
            .find(|p| p.id == path_id)
            .map(|p| p.path.clone())
            .unwrap();

        let db = app.db.clone();
        let extensions = app.db.safe_lock().get_video_extensions();

        app.scan_cancel_flag.store(true, Ordering::Relaxed);
        let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        app.scan_cancel_flag = cancel_flag.clone();

        app.scanning = true;
        app.scan_progress = 0.0;
        app.scan_status = crate::lang::t(app.lang, "scanning").to_string();
        app.thumbnail_progress = None;

        let (tx, rx) = std::sync::mpsc::channel();
        app.scan_complete_rx = Some(rx);

        std::thread::spawn(move || {
            let scanner = Scanner::new(db.clone(), extensions, cancel_flag);
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(scanner.scan_path(path_id, &path));

            match result {
                Ok(scan_result) => {
                    info!("Scan completed: {} files, {} need thumbnails", scan_result.count, scan_result.files_needing_thumbnails.len());
                    let _ = tx.send(crate::app::ScanResult {
                        path_id,
                        count: scan_result.count,
                        success: true,
                        message: String::new(),
                        files_needing_thumbnails: scan_result.files_needing_thumbnails,
                    });
                }
                Err(e) => {
                    log::error!("Scan failed: {}", e);
                    let _ = tx.send(crate::app::ScanResult {
                        path_id,
                        count: 0,
                        success: false,
                        message: e,
                        files_needing_thumbnails: Vec::new(),
                    });
                }
            }
        });

        if let Some(ctx) = &app.egui_ctx {
            ctx.request_repaint();
        }
    }

}

impl Default for MainWindow {
    fn default() -> Self { Self::new() }
}