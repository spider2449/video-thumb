use crate::app::SortBy;
use crate::App;
use crate::scanner::Scanner;
use eframe::egui;
use log::info;

pub struct MainWindow {}

impl MainWindow {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show_main(ctx: &egui::Context, app: &mut App) {
        let _available_width = ctx.input(|i| i.screen_rect.width()) - 40.0;
        let _cols = ((_available_width / app.thumbnail_size) as usize).max(1);

        // Toolbar
        egui::TopBottomPanel::top("toolbar")
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("+ 新增路徑").clicked() {
                        app.show_add_path = true;
                        app.reset_add_path_dialog();
                    }

                    let scan_text = if app.scanning {
                        format!("掃描中... {:.0}%", app.scan_progress * 100.0)
                    } else {
                        "🔄 更新資料庫".to_string()
                    };

                    ui.add_enabled_ui(!app.scanning, |ui| {
                        if ui.button(&scan_text).clicked() {
                            Self::start_scan(app);
                        }
                    });

                    ui.separator();

                    ui.menu_button("排序", |ui| {
                        if ui.selectable_label(app.sort_by == SortBy::Name, "名稱").clicked() {
                            app.sort_by = SortBy::Name;
                            app.invalidate_cache();
                            ui.close_menu();
                        }
                        if ui.selectable_label(app.sort_by == SortBy::Date, "日期").clicked() {
                            app.sort_by = SortBy::Date;
                            app.invalidate_cache();
                            ui.close_menu();
                        }
                        if ui.selectable_label(app.sort_by == SortBy::Rating, "評分").clicked() {
                            app.sort_by = SortBy::Rating;
                            app.invalidate_cache();
                            ui.close_menu();
                        }
                    });

                    ui.separator();

                    ui.label("搜尋:");
                    ui.text_edit_singleline(&mut app.search_query);

                    ui.separator();

                    ui.menu_button("評分篩選", |ui| {
                        if ui.selectable_label(app.rating_filter.is_none(), "全部").clicked() {
                            app.rating_filter = None;
                            app.invalidate_cache();
                            ui.close_menu();
                        }
                        for i in 1..=5 {
                            let label = format!("{} 星", i);
                            if ui.selectable_label(app.rating_filter == Some(i), &label).clicked() {
                                app.rating_filter = Some(i);
                                app.invalidate_cache();
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();

                    if !app.tags.is_empty() {
                        ui.menu_button("標籤篩選", |ui| {
                            if ui.selectable_label(app.tag_filter.is_none(), "全部").clicked() {
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

                    if ui.button("🏷️ 標籤管理").clicked() {
                        app.show_tag_manager = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Slider::new(&mut app.thumbnail_size, 80.0..=400.0).text("大小"));
                        ui.separator();

                        // Theme toggle button
                        let theme_icon = if app.current_theme == "light" { "☽" } else { "☀" };
                        if ui.button(theme_icon).clicked() {
                            app.current_theme = if app.current_theme == "light" {
                                "dark".to_string()
                            } else {
                                "light".to_string()
                            };
                            // Apply theme immediately
                            if app.current_theme == "light" {
                                ctx.set_visuals(egui::Visuals::light());
                            } else {
                                ctx.set_visuals(egui::Visuals::dark());
                            }
                            // Save preference
                            let db = app.db.blocking_lock();
                            let _ = db.set_setting("theme", &app.current_theme);
                        }

                        if ui.button("⚙️ 設定").clicked() {
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
                            let paths_snapshot: Vec<_> = app.watched_paths.clone();
                            for path in &paths_snapshot {
                                let selected = app.current_path_id == Some(path.id);
                                let button = egui::Button::new(&path.display_name).selected(selected);

                                if ui.add(button).clicked() {
                                    app.current_path_id = Some(path.id);
                                    app.selected_file_id = None;
                                    if !app.files.contains_key(&path.id) {
                                        app.load_files_for_path(path.id);
                                    }
                                }

                                if selected {
                                    if ui.button("×").clicked() {
                                        app.remove_path_confirm = Some((path.id, path.display_name.clone()));
                                    }
                                }
                            }
                        });
                    ui.separator();
                    if ui.button("+ 新增路徑").clicked() {
                        app.show_add_path = true;
                        app.reset_add_path_dialog();
                    }
                });
            });

        // Main content
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let files = app.get_current_files();

                egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for file in files {
                        Self::show_thumbnail_item(ui, &file, app.thumbnail_size, ctx, app);
                    }
            });
        });
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
        egui::Window::new("確認移除路徑")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(300.0);
                ui.add_space(8.0);
                ui.label(format!("確定要移除「{}」嗎？", display_name));
                ui.add_space(4.0);
                ui.label(egui::RichText::new("（只會移除記錄，不會刪除檔案）").small().color(egui::Color32::GRAY));
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("確認移除").clicked() {
                            let db = app.db.blocking_lock();
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
                        if ui.button("取消").clicked() {
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
                    let files = app.get_current_files();
                    ui.label(format!("共 {} 個檔案", files.len()));
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
            egui::Window::new("新增監控路徑")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_width(400.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label("資料夾路徑:");
                        ui.text_edit_singleline(&mut app.add_path_path);
                        if ui.button("選擇...").clicked() {
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
                        ui.label("顯示名稱:");
                        ui.text_edit_singleline(&mut app.add_path_display_name);
                    });

                    if let Some(error) = &app.add_path_error {
                        ui.add_space(8.0);
                        ui.colored_label(egui::Color32::RED, error);
                    }

                    ui.add_space(16.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("取消").clicked() {
                            app.show_add_path = false;
                            app.reset_add_path_dialog();
                        }
                        if ui.button("新增").clicked() {
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
    let thumb_rc: std::rc::Rc<std::cell::RefCell<Option<egui::Response>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
        let response = ui.allocate_ui([size, size + 50.0].into(), |ui| {
    let thumb_rc2 = thumb_rc.clone();
            ui.vertical(|ui| {
                let thumb_size_inner = size - 4.0;
                let thumb_rect = ui.allocate_rect(
                    egui::Rect::from_min_size(ui.cursor().min, [thumb_size_inner, thumb_size_inner].into()),
                    egui::Sense::click_and_drag(),
                );
            *thumb_rc2.borrow_mut() = Some(thumb_rect.clone());

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
                        if let Some(tex) = Self::load_tex(ctx, &path) {
                            let tex_size = tex.size_vec2();
                            let container_size = thumb_rect.rect.size();

                            // Calculate aspect-ratio-preserving size
                            let aspect_tex = tex_size.x / tex_size.y;
                            let aspect_container = container_size.x / container_size.y;

                            let (draw_w, draw_h) = if aspect_tex > aspect_container {
                                // Texture is wider than container - fit to width
                                let h = container_size.x / aspect_tex;
                                (container_size.x, h)
                            } else {
                                // Texture is taller than container - fit to height
                                let w = container_size.y * aspect_tex;
                                (w, container_size.y)
                            };

                            // Center the image within the container
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

                // Show full path under thumbnail
                ui.label(egui::RichText::new(&file.file_path).size(10.0).color(egui::Color32::LIGHT_GRAY));

                ui.horizontal_wrapped(|ui| {
                    for star in 1..=5 {
                        let color = if star <= file.rating { egui::Color32::from_rgb(255, 200, 0) } else { egui::Color32::from_gray(80) };
                        ui.label(egui::RichText::new(if star <= file.rating { "★" } else { "☆" }).color(color).size(14.0));
                    }
                });
            });
        });

        // Check thumb_rect for clicks since that's where click-and-drag sense was allocated
    let thumb_response = match &*thumb_rc.borrow() { Some(r) => r.clone(), None => return };

    if thumb_response.double_clicked() {
        log::info!("Double-click detected on: {}", file.file_name);
            let file_path = file.file_path.clone();
            let player = app.db.blocking_lock().get_setting("default_player")
                .ok().flatten().unwrap_or_default();
            let player_str: &str = if player.is_empty() { "" } else { &player };
            if let Some(ffp) = crate::utils::get_ffmpeg_path() {
                let gen = crate::thumbnail::ThumbnailGenerator::new(&crate::utils::get_data_dir(), &ffp);
                if let Err(e) = gen.play_video(&file_path, player_str) {
            log::error!("Failed to play video: {}", e);
        }
            }
        }

        if thumb_response.clicked() && !thumb_response.double_clicked() {
            app.selected_file_id = Some(file.id);
        }

        if thumb_response.secondary_clicked() {
        if app.context_menu.is_none() {
            let mut menu = crate::ui::ContextMenu::new();
            menu.file = Some(file.clone());
            menu.click_pos = thumb_response.interact_pointer_pos()
                .unwrap_or_else(|| response.response.rect.min);
            app.context_menu = Some(menu);
        } else {
            // Menu already open - just update file and position
            if let Some(ref mut menu) = app.context_menu {
                menu.file = Some(file.clone());
                menu.click_pos = response.response.interact_pointer_pos()
                    .unwrap_or_else(|| response.response.rect.min);
            }
        }
    }
    }

    fn load_tex(ctx: &egui::Context, path: &std::path::PathBuf) -> Option<egui::TextureHandle> {
        use std::collections::HashMap;
        use std::sync::Mutex;
        thread_local! { static CACHE: Mutex<HashMap<String, egui::TextureHandle>> = Mutex::new(HashMap::new()); }

        let key = path.to_string_lossy().to_string();
        CACHE.with(|c| {
            let mut cache = c.lock().ok()?;
            if let Some(h) = cache.get(&key) {
                return Some(h.clone());
            }
            let img = image::open(path).ok()?;
            let rgba = img.to_rgba8();
            let colors: Vec<egui::Color32> = rgba.pixels().map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])).collect();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_img = egui::ColorImage { size, pixels: colors };
            let h = ctx.load_texture(&key, color_img, egui::TextureOptions::LINEAR);
            cache.insert(key, h.clone());
            Some(h)
        })
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
        let extensions = app.db.blocking_lock().get_video_extensions();

        app.scanning = true;
        app.scan_progress = 0.0;
        app.scan_status = "掃描中...".to_string();
        app.thumbnail_progress = None;

        let (tx, rx) = std::sync::mpsc::channel();
        app.scan_complete_rx = Some(rx);

        std::thread::spawn(move || {
            let scanner = Scanner::new(db.clone(), extensions);
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
    }

}

impl Default for MainWindow {
    fn default() -> Self { Self::new() }
}