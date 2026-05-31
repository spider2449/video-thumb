use crate::db::{Database, FileEntry, WatchedPath};
use crate::lang::Lang;
use crate::ui::{MainWindow, SettingsDialog, TagManager};
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct App {
    pub db: Arc<Mutex<Database>>,
    pub watched_paths: Vec<WatchedPath>,
    pub files: HashMap<i64, Arc<Vec<FileEntry>>>,
    pub filter_cache: Option<(i64, SortBy, String, Option<i32>, Option<i64>, Arc<Vec<FileEntry>>)>,
    pub file_tag_map: HashMap<i64, Vec<i64>>, // file_id -> tag_ids
    pub tags: Vec<crate::db::Tag>,
    pub current_path_id: Option<i64>,
    pub selected_file_id: Option<i64>,
    pub thumbnail_size: f32,
    pub search_query: String,
    pub rating_filter: Option<i32>,
    pub tag_filter: Option<i64>,
    pub sort_by: SortBy,
    pub show_settings: bool,
    pub show_add_path: bool,
    pub show_tag_manager: bool,
    pub scanning: bool,
    pub loading_files: bool,
    pub scan_progress: f32,
    pub scan_status: String,
    pub scan_complete_tx: Option<std::sync::mpsc::Sender<ScanResult>>,
    pub scan_complete_rx: Option<std::sync::mpsc::Receiver<ScanResult>>,
    pub thumbnail_progress: Option<(usize, usize)>,
    pub thumb_progress_rx: Option<std::sync::mpsc::Receiver<usize>>,
    pub thumb_done_rx: Option<std::sync::mpsc::Receiver<i64>>,
    // Dialog state
    pub add_path_path: String,
    pub add_path_display_name: String,
    pub add_path_error: Option<String>,
    // Dialog instances (stored to preserve state across frames)
    pub settings_dialog: Option<SettingsDialog>,
    pub tag_manager: Option<TagManager>,
    pub context_menu: Option<crate::ui::ContextMenu>,
    pub screenshot_dialog: Option<crate::ui::ScreenshotDialog>,
    // Remove path confirmation dialog
    pub remove_path_confirm: Option<(i64, String)>, // (path_id, display_name)
    // Console visibility (for F12 toggle)
    pub console_visible: bool,
    pub cache_invalidated: bool,
    // Theme storage
    pub current_theme: String,
    pub theme_loaded: bool,
    // Scroll target for keyboard navigation
    pub scroll_to_idx: Option<usize>,
    // Language
    pub lang: Lang,
}

pub struct ScanResult {
    pub path_id: i64,
    pub count: usize,
    pub success: bool,
    pub message: String,
    pub files_needing_thumbnails: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Date,
    Rating,
}

impl Default for SortBy {
    fn default() -> Self { SortBy::Name }
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let data_dir = crate::utils::get_data_dir();
        std::fs::create_dir_all(&data_dir).ok();
        std::fs::create_dir_all(data_dir.join("thumbs")).ok();

        let db = Database::new(&data_dir.join("data.db"))
            .expect("Failed to initialize database");
        let db = Arc::new(Mutex::new(db));

        let watched_paths = db.lock().unwrap().get_all_paths().unwrap_or_default();
        let tags = db.lock().unwrap().get_all_tags().unwrap_or_default();
        let lang = Lang::from_setting(
            &db.lock().unwrap().get_setting("language").ok().flatten().unwrap_or_else(|| "zh_tw".to_string())
        );

        let mut app = Self {
            db,
            watched_paths,
            files: HashMap::new(),
            file_tag_map: HashMap::new(),
            tags,
            current_path_id: None,
            selected_file_id: None,
            thumbnail_size: 200.0,
            search_query: String::new(),
            rating_filter: None,
            tag_filter: None,
            sort_by: SortBy::Name,
            show_settings: false,
            show_add_path: false,
            show_tag_manager: false,
            scanning: false,
            loading_files: false,
            scan_progress: 0.0,
            scan_status: String::new(),
            scan_complete_tx: None,
            scan_complete_rx: None,
            add_path_path: String::new(),
            add_path_display_name: String::new(),
            add_path_error: None,
        settings_dialog: None,
            tag_manager: None,
            context_menu: None,
            screenshot_dialog: None,
            remove_path_confirm: None,
            console_visible: true,
            cache_invalidated: false,
            current_theme: "dark".to_string(),
            theme_loaded: false,
            thumbnail_progress: None,
            thumb_progress_rx: None,
            thumb_done_rx: None,
            filter_cache: None,
            scroll_to_idx: None,
            lang,
        };

        if app.watched_paths.is_empty() {
            app.show_add_path = true;
        } else {
            app.current_path_id = Some(app.watched_paths[0].id);
            app.load_files_for_path(app.watched_paths[0].id);
        }

        app
    }

    pub fn load_files_for_path(&mut self, path_id: i64) {
        self.loading_files = true;
        let db_guard = self.db.lock().unwrap();
        match db_guard.get_files_for_path(path_id) {
            Ok(files) => {
                log::info!("Loaded {} files for path_id={}", files.len(), path_id);
                // Clear stale tag map entries for this path's previous files
                if let Some(old_files) = self.files.get(&path_id) {
                    for file in old_files.iter() {
                        self.file_tag_map.remove(&file.id);
                    }
                }
                // Build tag map with a single batch query
                match db_guard.get_all_file_tags(path_id) {
                    Ok(tag_map) => {
                        self.file_tag_map.extend(tag_map);
                    }
                    Err(e) => {
                        log::warn!("Failed to load file tags: {:?}", e);
                    }
                }
                self.files.insert(path_id, Arc::new(files));
            }
            Err(e) => {
                log::error!("Failed to load files for path_id={}: {:?}", path_id, e);
            }
        }
        self.filter_cache = None;
        self.loading_files = false;
    }

    pub fn get_current_files(&mut self) -> Arc<Vec<FileEntry>> {
        let path_id = match self.current_path_id {
            Some(id) => id,
            None => {
                if let Some(first) = self.watched_paths.first() {
                    first.id
                } else {
                    return Arc::new(vec![]);
                }
            }
        };

        // Check filter cache
        if let Some(ref cache) = self.filter_cache {
            if cache.0 == path_id
                && cache.1 == self.sort_by
                && cache.2 == self.search_query
                && cache.3 == self.rating_filter
                && cache.4 == self.tag_filter
            {
                return Arc::clone(&cache.5);
            }
        }

        // Get base files from cache or DB
        let base_files = if let Some(cached) = self.files.get(&path_id) {
            Arc::clone(cached)
        } else {
            let db_guard = self.db.lock().unwrap();
            let files = db_guard.get_files_for_path(path_id).unwrap_or_default();
            let files = Arc::new(files);
            self.files.insert(path_id, Arc::clone(&files));
            files
        };

        // Clone for mutation (filtering + sorting)
        let mut files: Vec<FileEntry> = (*base_files).clone();

        // Apply filters in-memory
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            files.retain(|f| f.file_path.to_lowercase().contains(&query));
        }

        if let Some(rating) = self.rating_filter {
            files.retain(|f| f.rating == rating);
        }

        if let Some(tag_id) = self.tag_filter {
            files.retain(|f| {
                self.file_tag_map.get(&f.id)
                    .map(|tags| tags.contains(&tag_id))
                    .unwrap_or(false)
            });
        }

        match self.sort_by {
            SortBy::Name => files.sort_by(|a, b| a.file_path.cmp(&b.file_path)),
            SortBy::Date => files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at)),
            SortBy::Rating => files.sort_by(|a, b| b.rating.cmp(&a.rating)),
        }

        let result = Arc::new(files);
        self.filter_cache = Some((path_id, self.sort_by, self.search_query.clone(), self.rating_filter, self.tag_filter, Arc::clone(&result)));
        result
    }

    pub fn invalidate_cache(&mut self) {
        self.cache_invalidated = true;
    }

    pub fn refresh_cache_if_needed(&mut self) {
        if self.cache_invalidated {
            if let Some(path_id) = self.current_path_id {
                self.load_files_for_path(path_id);
            }
            self.cache_invalidated = false;
        }
    }

    pub fn play_video_file(&self, file_path: &str) {
        let player = self.db.lock().unwrap().get_setting("default_player")
            .ok().flatten().unwrap_or_default();
        let player_str: &str = if player.is_empty() { "" } else { &player };
        if let Some(ffp) = crate::utils::get_ffmpeg_path() {
            let gen = crate::thumbnail::ThumbnailGenerator::new(&crate::utils::get_data_dir(), &ffp, self.db.clone());
            if let Err(e) = gen.play_video(file_path, player_str) {
                log::error!("Failed to play video: {}", e);
            }
        }
    }

    pub fn add_path(&mut self) {
        if self.add_path_path.is_empty() {
            self.add_path_error = Some(crate::lang::t(self.lang, "select_folder_path").to_string());
            return;
        }
        let path = std::path::Path::new(&self.add_path_path);
        if !path.exists() {
            self.add_path_error = Some(crate::lang::t(self.lang, "path_not_exist").to_string());
            return;
        }
        if !path.is_dir() {
            self.add_path_error = Some(crate::lang::t(self.lang, "path_not_dir").to_string());
            return;
        }

        let display_name = if self.add_path_display_name.is_empty() {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| self.add_path_path.clone())
        } else {
            self.add_path_display_name.clone()
        };

        let path_str = self.add_path_path.clone();
        let db = self.db.lock().unwrap();

        match db.path_exists(&path_str) {
            Ok(true) => {
                self.add_path_error = Some(crate::lang::t(self.lang, "path_already_exists").to_string());
            }
            Ok(false) => {
                match db.add_path(&path_str, &display_name) {
                    Ok(id) => {
                        drop(db);
                        self.watched_paths.push(crate::db::WatchedPath {
                            id,
                            path: path_str,
                            display_name,
                            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        });
                        self.current_path_id = Some(id);
                        self.show_add_path = false;
                        self.add_path_path.clear();
                        self.add_path_display_name.clear();
                        self.add_path_error = None;
                        self.load_files_for_path(id);
                    }
                    Err(e) => {
                        self.add_path_error = Some(format!("{}{}", crate::lang::t(self.lang, "add_failed_prefix"), e));
                    }
                }
            }
            Err(e) => {
                self.add_path_error = Some(format!("{}{}", crate::lang::t(self.lang, "query_failed_prefix"), e));
            }
        }
    }

    pub fn reset_add_path_dialog(&mut self) {
        self.add_path_path.clear();
        self.add_path_display_name.clear();
        self.add_path_error = None;
    }

    pub fn apply_theme(&self, ctx: &egui::Context) {
        if self.current_theme == "light" {
            ctx.set_visuals(egui::Visuals::light());
        } else {
            ctx.set_visuals(egui::Visuals::dark());
        }
    }

    pub fn start_thumbnail_generation(&mut self, files: Vec<String>, path_id: i64) {
        let total = files.len();
        let db = self.db.clone();
        let data_dir = crate::utils::get_data_dir();
        let (progress_tx, progress_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();

        self.thumbnail_progress = Some((0, total));
        self.scan_status = format!("{}0{}{}", crate::lang::t(self.lang, "generating_thumbs_prefix"), crate::lang::t(self.lang, "generating_thumbs_sep"), total);

        // Drop any orphaned channels from previous generation
        self.thumb_progress_rx = None;
        self.thumb_done_rx = None;

        // Store channels for polling
        self.thumb_progress_rx = Some(progress_rx);
        self.thumb_done_rx = Some(done_rx);

        std::thread::spawn(move || {
            let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() else {
                log::error!("ffmpeg not found, skipping thumbnail generation");
                let _ = done_tx.send(path_id);
                return;
            };
            let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path, db.clone());

            // Build path->id map once to avoid O(n²) lookups
            let db_guard = db.lock().unwrap();
            let path_to_id: std::collections::HashMap<String, i64> = if let Ok(all_files) = db_guard.get_files_for_path(path_id) {
                all_files.into_iter().map(|f| (f.file_path, f.id)).collect()
            } else {
                std::collections::HashMap::new()
            };
            drop(db_guard);

            for (i, file_path) in files.iter().enumerate() {
                let file_id = path_to_id.get(file_path).copied();
                match gen.generate_thumbnail_at_frame(file_path, 300, file_id) {
                    Ok(thumb_path) => {
                        if let Some(file_id) = file_id {
                            let db_guard = db.lock().unwrap();
                            let _ = db_guard.update_thumbnail(file_id, &thumb_path);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to generate thumbnail for {}: {}", file_path, e);
                    }
                }
                let _ = progress_tx.send(i + 1);
            }
            let _ = done_tx.send(path_id);
        });
    }

    fn update_impl(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.scanning || self.thumbnail_progress.is_some() || self.loading_files {
            ctx.set_cursor_icon(egui::CursorIcon::Wait);
        }

        // Load and apply theme on first frame
        if !self.theme_loaded {
            let db_guard = self.db.lock().unwrap();
            self.current_theme = db_guard.get_setting("theme")
                .ok()
                .flatten()
                .unwrap_or_else(|| "dark".to_string());
            drop(db_guard);
            self.apply_theme(ctx);
            self.theme_loaded = true;
        }

        // Toggle console with F12
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            self.console_visible = !self.console_visible;
            crate::toggle_console_window(self.console_visible);
        }

        // Check for scan completion
        if let Some(rx) = self.scan_complete_rx.take() {
            if let Ok(result) = rx.try_recv() {
                self.scanning = false;
                if result.success {
                    self.scan_status = format!("{}{}{}", crate::lang::t(self.lang, "done_found_prefix"), result.count, crate::lang::t(self.lang, "done_found_suffix"));
                    // Always reload cache immediately after scan
                    self.load_files_for_path(result.path_id);
                    // Start thumbnail generation in background if needed
                    if !result.files_needing_thumbnails.is_empty() {
                        self.start_thumbnail_generation(result.files_needing_thumbnails, result.path_id);
                    }
                } else {
                    self.scan_status = format!("{}{}", crate::lang::t(self.lang, "failed_prefix"), result.message);
                }
                // Force UI update by requesting a repaint
                ctx.request_repaint();
            } else {
                self.scan_complete_rx = Some(rx);
            }
        }

        // Poll thumbnail progress
        if let Some(rx) = self.thumb_progress_rx.as_ref() {
            let mut needs_repaint = false;
            while let Ok(completed) = rx.try_recv() {
                if let Some((_, total)) = self.thumbnail_progress {
                    self.thumbnail_progress = Some((completed, total));
                    self.scan_status = format!("{}{}{}{}", crate::lang::t(self.lang, "generating_thumbs_prefix"), completed, crate::lang::t(self.lang, "generating_thumbs_sep"), total);
                    needs_repaint = true;
                }
            }
            if needs_repaint {
                ctx.request_repaint();
            }
        }

        // Check for thumbnail generation completion
        if let Some(rx) = self.thumb_done_rx.take() {
            if let Ok(path_id) = rx.try_recv() {
                self.thumbnail_progress = None;
                self.scan_status = format!("{}{}{}", crate::lang::t(self.lang, "done_found_prefix"), self.files.get(&path_id).map(|f| f.len()).unwrap_or(0), crate::lang::t(self.lang, "done_found_suffix"));
                self.load_files_for_path(path_id);
                ctx.request_repaint();
            } else {
                self.thumb_done_rx = Some(rx);
            }
        }

        self.refresh_cache_if_needed();

        MainWindow::show_main(ctx, self);

        // Keyboard navigation for selected video
        if let Some(selected_id) = self.selected_file_id {
            let current_files = self.get_current_files();
            if let Some(selected_idx) = current_files.iter().position(|f| f.id == selected_id) {
                let screen_width = ctx.input(|i| i.screen_rect.width());
                let items_per_row = ((screen_width - 40.0) / self.thumbnail_size).max(1.0) as usize;

                let new_idx = if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                    Some(selected_idx.saturating_add(1).min(current_files.len().saturating_sub(1)))
                } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                    Some(selected_idx.saturating_sub(1))
                } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    Some((selected_idx + items_per_row).min(current_files.len().saturating_sub(1)))
                } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    Some(selected_idx.saturating_sub(items_per_row))
                } else if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
                    Some((selected_idx + items_per_row).min(current_files.len().saturating_sub(1)))
                } else if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
                    Some(selected_idx.saturating_sub(items_per_row))
                } else if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
                    Some(0)
                } else if ctx.input(|i| i.key_pressed(egui::Key::End)) {
                    Some(current_files.len().saturating_sub(1))
                } else if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(file) = current_files.get(selected_idx) {
                        self.play_video_file(&file.file_path);
                    }
                    None
                } else {
                    None
                };

                if let Some(idx) = new_idx {
                    self.selected_file_id = Some(current_files[idx].id);
                    self.scroll_to_idx = Some(idx);
                    ctx.request_repaint();
                }
            } else {
                // Selected file no longer in the current list (filtered out)
                self.selected_file_id = None;
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.update_impl(ctx, frame);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Window state will be saved via store_to_db call if available
    }

}