use crate::db::{Database, FileEntry, WatchedPath};
use crate::ui::{MainWindow, SettingsDialog, TagManager};
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
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
    // Dialog state
    pub add_path_path: String,
    pub add_path_display_name: String,
    pub add_path_error: Option<String>,
    // Dialog instances (stored to preserve state across frames)
    pub settings_dialog: Option<SettingsDialog>,
    pub tag_manager: Option<TagManager>,
    pub context_menu: Option<crate::ui::ContextMenu>,
    pub screenshot_dialog: Option<crate::ui::ScreenshotDialog>,
}

pub struct ScanResult {
    pub path_id: i64,
    pub count: usize,
    pub success: bool,
    pub message: String,
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

pub fn get_data_dir() -> PathBuf {
    crate::config::Config::load().get_data_dir()
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let data_dir = get_data_dir();
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

        // Always read directly from database to get latest data
        let db_guard = self.db.blocking_lock();
        let mut files = db_guard.get_files_for_path(path_id).unwrap_or_default();
        drop(db_guard);

        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            files.retain(|f| f.file_name.to_lowercase().contains(&query));
        }

        if let Some(rating) = self.rating_filter {
            files.retain(|f| f.rating == rating);
        }

        if let Some(tag_id) = self.tag_filter {
            let db_guard = self.db.blocking_lock();
            files.retain(|f| {
                if let Ok(tags) = db_guard.get_tags_for_file(f.id) {
                    tags.iter().any(|t| t.id == tag_id)
                } else {
                    false
                }
            });
        }

        match self.sort_by {
            SortBy::Name => files.sort_by(|a, b| a.file_path.cmp(&b.file_path)),
            SortBy::Date => files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at)),
            SortBy::Rating => files.sort_by(|a, b| b.rating.cmp(&a.rating)),
        }

        files
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

    fn update_impl(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for scan completion
        if let Some(rx) = self.scan_complete_rx.take() {
            if let Ok(result) = rx.try_recv() {
                self.scanning = false;
                self.scan_status = if result.success {
                    format!("完成！找到 {} 個檔案", result.count)
                } else {
                    format!("失敗: {}", result.message)
                };
                // Force UI update by requesting a repaint
                ctx.request_repaint();
            } else {
                self.scan_complete_rx = Some(rx);
            }
        }

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