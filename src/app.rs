use crate::db::{Database, FileEntry, WatchedPath};
use crate::ui::{MainWindow, SettingsDialog, TagManager};
use eframe::egui;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct App {
    pub db: Arc<Mutex<Database>>,
    pub watched_paths: Vec<WatchedPath>,
    pub files: HashMap<i64, Vec<FileEntry>>,
    pub tags: Vec<crate::db::Tag>,
    pub current_path_id: Option<i64>,
    pub thumbnail_size: f32,
    pub search_query: String,
    pub rating_filter: Option<i32>,
    pub tag_filter: Option<i64>,
    pub sort_by: SortBy,
    pub show_settings: bool,
    pub show_add_path: bool,
    pub show_tag_manager: bool,
    pub scanning: bool,
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

        let watched_paths = db.blocking_lock().get_all_paths().unwrap_or_default();
        let tags = db.blocking_lock().get_all_tags().unwrap_or_default();

        let mut app = Self {
            db,
            watched_paths,
            files: HashMap::new(),
            tags,
            current_path_id: None,
            thumbnail_size: 200.0,
            search_query: String::new(),
            rating_filter: None,
            tag_filter: None,
            sort_by: SortBy::Name,
            show_settings: false,
            show_add_path: false,
            show_tag_manager: false,
            scanning: false,
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
            thumbnail_progress: None,
            thumb_progress_rx: None,
            thumb_done_rx: None,
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
        let db_guard = self.db.blocking_lock();
        if let Ok(files) = db_guard.get_files_for_path(path_id) {
            self.files.insert(path_id, files);
        }
    }

    pub fn get_current_files(&self) -> Vec<FileEntry> {
        let path_id = match self.current_path_id {
            Some(id) => id,
            None => {
                if let Some(first) = self.watched_paths.first() {
                    first.id
                } else {
                    return vec![];
                }
            }
        };

        // Get files from cache or DB
        let mut files = if let Some(cached) = self.files.get(&path_id) {
            cached.clone()
        } else {
            Vec::new()
        };

        // Apply filters in-memory
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            files.retain(|f| f.file_path.to_lowercase().contains(&query));
        }

        if let Some(rating) = self.rating_filter {
            files.retain(|f| f.rating == rating);
        }

        if let Some(tag_id) = self.tag_filter {
            // Tag filter still needs DB lookup per file (could be optimized later)
            // For now, skip if no cache to avoid blocking
            if self.files.contains_key(&path_id) {
                let db_guard = self.db.blocking_lock();
                files.retain(|f| {
                    if let Ok(tags) = db_guard.get_tags_for_file(f.id) {
                        tags.iter().any(|t| t.id == tag_id)
                    } else {
                        false
                    }
                });
            }
        }

        match self.sort_by {
            SortBy::Name => files.sort_by(|a, b| a.file_path.cmp(&b.file_path)),
            SortBy::Date => files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at)),
            SortBy::Rating => files.sort_by(|a, b| b.rating.cmp(&a.rating)),
        }

        files
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

    pub fn add_path(&mut self) {
        if self.add_path_path.is_empty() {
            self.add_path_error = Some("請選擇資料夾路徑".to_string());
            return;
        }
        let path = std::path::Path::new(&self.add_path_path);
        if !path.exists() {
            self.add_path_error = Some("路徑不存在".to_string());
            return;
        }
        if !path.is_dir() {
            self.add_path_error = Some("路徑不是有效的資料夾".to_string());
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
        let db = self.db.blocking_lock();

        match db.path_exists(&path_str) {
            Ok(true) => {
                self.add_path_error = Some("此路徑已存在".to_string());
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
                        self.add_path_error = Some(format!("新增失敗: {}", e));
                    }
                }
            }
            Err(e) => {
                self.add_path_error = Some(format!("查詢失敗: {}", e));
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
        self.scan_status = format!("生成縮圖中... 0/{}", total);

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
            let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path);

            // Build path->id map once to avoid O(n²) lookups
            let db_guard = db.blocking_lock();
            let path_to_id: std::collections::HashMap<String, i64> = if let Ok(all_files) = db_guard.get_files_for_path(path_id) {
                all_files.into_iter().map(|f| (f.file_path, f.id)).collect()
            } else {
                std::collections::HashMap::new()
            };
            drop(db_guard);

            for (i, file_path) in files.iter().enumerate() {
                match gen.generate_thumbnail_at_frame(file_path, 300) {
                    Ok(thumb_path) => {
                        if let Some(&file_id) = path_to_id.get(file_path) {
                            let db_guard = db.blocking_lock();
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
        // Load and apply theme on first frame
        if self.current_theme == "dark" {
            let db_guard = self.db.blocking_lock();
            self.current_theme = db_guard.get_setting("theme")
                .ok()
                .flatten()
                .unwrap_or_else(|| "dark".to_string());
            drop(db_guard);
            self.apply_theme(ctx);
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
                    self.scan_status = format!("完成！找到 {} 個檔案", result.count);
                    // Start thumbnail generation in background
                    if !result.files_needing_thumbnails.is_empty() {
                        self.start_thumbnail_generation(result.files_needing_thumbnails, result.path_id);
                    } else {
                        self.load_files_for_path(result.path_id);
                    }
                } else {
                    self.scan_status = format!("失敗: {}", result.message);
                }
                // Force UI update by requesting a repaint
                ctx.request_repaint();
            } else {
                self.scan_complete_rx = Some(rx);
            }
        }

        // Poll thumbnail progress
        if let Some(rx) = self.thumb_progress_rx.as_ref() {
            while let Ok(completed) = rx.try_recv() {
                if let Some((_, total)) = self.thumbnail_progress {
                    self.thumbnail_progress = Some((completed, total));
                    self.scan_status = format!("生成縮圖中... {}/{}", completed, total);
                    ctx.request_repaint();
                }
            }
        }

        // Check for thumbnail generation completion
        if let Some(rx) = self.thumb_done_rx.take() {
            if let Ok(path_id) = rx.try_recv() {
                self.thumbnail_progress = None;
                self.scan_status = format!("完成！找到 {} 個檔案", self.files.get(&path_id).map(|f| f.len()).unwrap_or(0));
                self.load_files_for_path(path_id);
                ctx.request_repaint();
            } else {
                self.thumb_done_rx = Some(rx);
            }
        }

        self.refresh_cache_if_needed();

        MainWindow::show_main(ctx, self);
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