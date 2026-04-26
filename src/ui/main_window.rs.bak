use crate::app::SortBy;
use crate::ui::ContextMenu;
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
                ui.horizontal(|ui| {
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
                            ui.close_menu();
                        }
                        if ui.selectable_label(app.sort_by == SortBy::Date, "日期").clicked() {
                            app.sort_by = SortBy::Date;
                            ui.close_menu();
                        }
                        if ui.selectable_label(app.sort_by == SortBy::Rating, "評分").clicked() {
                            app.sort_by = SortBy::Rating;
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
                            ui.close_menu();
                        }
                        for i in 1..=5 {
                            let label = format!("{} 星", i);
                            if ui.selectable_label(app.rating_filter == Some(i), &label).clicked() {
                                app.rating_filter = Some(i);
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();

                    if !app.tags.is_empty() {
                        ui.menu_button("標籤篩選", |ui| {
                            if ui.selectable_label(app.tag_filter.is_none(), "全部").clicked() {
                                app.tag_filter = None;
                                ui.close_menu();
                            }
                            for tag in &app.tags {
                                let color = Self::hex_to_color(&tag.color);
                                ui.colored_label(color, &tag.name);
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
                        if ui.button("⚙️ 設定").clicked() {
                            app.show_settings = true;
                        }
                    });
                });
            });

        // Tab bar
        egui::TopBottomPanel::top("tab_bar")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    egui::ScrollArea::horizontal()
                        .drag_to_scroll(false)
                        .show(ui, |ui| {
                            let paths_snapshot: Vec<_> = app.watched_paths.clone();
                            let mut close_request: Option<i64> = None;
                            for path in &paths_snapshot {
                                let selected = app.current_path_id == Some(path.id);
                                let button = egui::Button::new(&path.display_name).selected(selected);

                                if ui.add(button).clicked() {
                                    app.current_path_id = Some(path.id);
                                    if !app.files.contains_key(&path.id) {
                                        app.load_files_for_path(path.id);
                                    }
                                }

                                if selected {
                                    if ui.button("×").clicked() {
                                        close_request = app.current_path_id;
                                    }
                                }
                            }

                            if let Some(id) = close_request {
                                let db = app.db.blocking_lock();
                                let should_remove = db.remove_path(id).is_ok();
                                drop(db);
                                if should_remove {
                                    app.watched_paths.retain(|p| p.id != id);
                                    app.files.remove(&id);
                                    app.current_path_id = app.watched_paths.first().map(|p| p.id);
                                    if let Some(new_id) = app.current_path_id {
                                        app.load_files_for_path(new_id);
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

    // Status bar
        egui::TopBottomPanel::bottom("status_bar")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
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

                    ui.horizontal(|ui| {
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
                    ui.horizontal(|ui| {
                        ui.label("顯示名稱:");
                        ui.text_edit_singleline(&mut app.add_path_display_name);
                    });

                    if let Some(error) = &app.add_path_error {
                        ui.add_space(8.0);
                        ui.colored_label(egui::Color32::RED, error);
                    }

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
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

                if let Some(thumb_path) = &file.thumbnail_path {
                    let path = std::path::PathBuf::from(thumb_path);
                    if path.exists() {
                        if let Some(tex) = Self::load_tex(ctx, &path) {
                            let img_rect = egui::Rect::from_min_size(thumb_rect.rect.min, thumb_rect.rect.size());
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

                ui.label(egui::RichText::new(Self::trunc_name(&file.file_name, 20)).size(12.0).color(egui::Color32::WHITE));

                ui.horizontal(|ui| {
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
            if let Some(ffp) = Self::get_ffmpeg() {
                let gen = crate::thumbnail::ThumbnailGenerator::new(&Self::get_data_dir(), &ffp);
                if let Err(e) = gen.play_video(&file_path, player_str) {
            log::error!("Failed to play video: {}", e);
        }
            }
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

        let path_id = app.current_path_id.or_else(|| app.watched_paths.first().map(|p| p.id)).unwrap();
        let path = app.watched_paths.iter().find(|p| p.id == path_id).map(|p| p.path.clone()).unwrap();

        let db = app.db.clone();
        let extensions = app.db.blocking_lock().get_video_extensions();

        app.scanning = true;
        app.scan_progress = 0.0;
        app.scan_status = "掃描中...".to_string();

        // Create channel for scan result
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
                Ok(count) => {
                    info!("Scan completed: {} files", count);
                    let _ = tx.send(crate::app::ScanResult {
                        path_id,
                        count,
                        success: true,
                        message: String::new(),
                    });
                }
                Err(e) => {
                    log::error!("Scan failed: {}", e);
                    let _ = tx.send(crate::app::ScanResult {
                        path_id,
                        count: 0,
                        success: false,
                        message: e,
                    });
                }
            }
        });
    }

    pub fn get_ffmpeg() -> Option<std::path::PathBuf> {
        let dir = Self::get_data_dir().join("ffmpeg");
        #[cfg(target_os = "windows")] {
            let p = dir.join("ffmpeg.exe");
            if p.exists() { return Some(p); }
            if std::process::Command::new("where").arg("ffmpeg").output().map(|o| o.status.success()).unwrap_or(false) {
                return Some(std::path::PathBuf::from("ffmpeg"));
            }
        }
        #[cfg(not(target_os = "windows"))] {
            let p = dir.join("ffmpeg");
            if p.exists() { return Some(p); }
            return Some(std::path::PathBuf::from("ffmpeg"));
        }
        None
    }

    fn hex_to_color(hex: &str) -> egui::Color32 {
        let hex = hex.trim_start_matches('#');
        if hex.len() >= 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
            egui::Color32::from_rgb(r, g, b)
        } else { egui::Color32::GRAY }
    }

    fn trunc_name(name: &str, max: usize) -> String {
        if name.chars().count() <= max { name.to_string() }
        else {
            let ext = std::path::Path::new(name).extension()
                .map(|e| e.len())
                .unwrap_or(0);
            let char_count = name.chars().count();
            let n = max.saturating_sub(ext + 4);

            if n > 0 && ext > 0 {
                let prefix: String = name.chars().take(n).collect();
                let suffix_start = char_count.saturating_sub(ext);
                let suffix: String = name.chars().skip(suffix_start).collect();
                format!("{}...{}", prefix, suffix)
            } else {
                name.chars().take(max).collect::<String>() + "..."
            }
        }
    }

    fn get_data_dir() -> std::path::PathBuf {
        dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("video-thumb-viewer")
    }
}

impl Default for MainWindow {
    fn default() -> Self { Self::new() }
}