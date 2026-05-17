# Phase 1: Incremental Performance Optimization

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the worst performance bottlenecks for 1000+ videos with minimal architectural change.

**Architecture:** Lazy-load files per tab, separate thumbnail generation from scanning, consolidate duplicated utilities, enable SQLite WAL mode, remove dead code.

**Tech Stack:** Rust, egui/eframe, rusqlite, tokio, ffmpeg subprocess, walkdir.

---

### Task 1: Create Shared Utilities Module

**Files:**
- Create: `src/utils.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/utils.rs`**

```rust
use std::path::PathBuf;

/// Get the effective data directory from config or system default
pub fn get_data_dir() -> PathBuf {
    crate::config::Config::load().get_data_dir()
}

/// Find ffmpeg executable: first in data_dir/ffmpeg/, then in PATH
pub fn get_ffmpeg_path() -> Option<PathBuf> {
    let ffmpeg_dir = get_data_dir().join("ffmpeg");

    #[cfg(target_os = "windows")]
    {
        let path = ffmpeg_dir.join("ffmpeg.exe");
        if path.exists() {
            return Some(path);
        }
        if std::process::Command::new("where")
            .arg("ffmpeg")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .is_some()
        {
            return Some(PathBuf::from("ffmpeg"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let path = ffmpeg_dir.join("ffmpeg");
        if path.exists() {
            return Some(path);
        }
        return Some(PathBuf::from("ffmpeg"));
    }

    None
}

/// Parse a hex color string (with or without #) into egui::Color32
pub fn hex_to_color(hex: &str) -> egui::Color32 {
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
```

- [ ] **Step 2: Add `mod utils;` to `src/main.rs`**

Add this line to the module declarations at the top of `src/main.rs`, after the existing `mod` declarations:

```rust
mod utils;
```

The top of `src/main.rs` should now read:

```rust
mod app;
mod config;
mod db;
mod scanner;
mod thumbnail;
mod ui;
mod utils;
```

- [ ] **Step 3: Remove `get_data_dir()` from `src/app.rs`**

Remove this function from `src/app.rs` (lines 63-65):

```rust
pub fn get_data_dir() -> PathBuf {
    crate::config::Config::load().get_data_dir()
}
```

Replace all `get_data_dir()` calls in `src/app.rs` with `crate::utils::get_data_dir()`.

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add src/utils.rs src/main.rs src/app.rs
git commit -m "refactor: create shared utils module with get_data_dir, get_ffmpeg_path, hex_to_color"
```

---

### Task 2: Replace Duplicated `get_ffmpeg_path()` and `get_data_dir()` in UI Files

**Files:**
- Modify: `src/ui/main_window.rs`
- Modify: `src/ui/context_menu.rs`
- Modify: `src/ui/screenshot_dialog.rs`
- Modify: `src/ui/tag_manager.rs`
- Modify: `src/scanner/mod.rs`

Note: `thumbnail_grid.rs` is NOT modified here — it will be deleted in Task 3.

- [ ] **Step 1: Remove duplicated functions from `src/ui/main_window.rs`**

Delete these functions from `src/ui/main_window.rs`:
- `get_ffmpeg()` (lines 523-538)
- `get_data_dir()` (lines 550-552)
- `hex_to_color()` (lines 540-548)

Replace all internal calls:
- `Self::get_ffmpeg()` → `crate::utils::get_ffmpeg_path()`
- `Self::get_data_dir()` → `crate::utils::get_data_dir()`
- `Self::hex_to_color(...)` → `crate::utils::hex_to_color(...)`

- [ ] **Step 2: Remove duplicated functions from `src/ui/context_menu.rs`**

Delete these functions from `src/ui/context_menu.rs`:
- `get_ffmpeg_path()` (lines 377-400)
- `get_data_dir()` (lines 402-404)

Replace all internal calls:
- `get_ffmpeg_path()` → `crate::utils::get_ffmpeg_path()`
- `get_data_dir()` → `crate::utils::get_data_dir()`

- [ ] **Step 3: Remove duplicated functions from `src/ui/screenshot_dialog.rs`**

Delete these functions from `src/ui/screenshot_dialog.rs`:
- `get_ffprobe()` (lines 56-78)
- `get_ffmpeg()` (lines 80-102)

Replace all internal calls:
- `Self::get_ffmpeg()` → `crate::utils::get_ffmpeg_path()`
- For ffprobe: use the same pattern but with `ffprobe` instead of `ffmpeg` — create a `get_ffprobe_path()` helper in `utils.rs`:

Add to `src/utils.rs`:

```rust
/// Find ffprobe executable: first in data_dir/ffmpeg/, then in PATH
pub fn get_ffprobe_path() -> Option<PathBuf> {
    let ffmpeg_dir = get_data_dir().join("ffmpeg");

    #[cfg(target_os = "windows")]
    {
        let path = ffmpeg_dir.join("ffprobe.exe");
        if path.exists() {
            return Some(path);
        }
        if std::process::Command::new("where")
            .arg("ffprobe")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .is_some()
        {
            return Some(PathBuf::from("ffprobe"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let path = ffmpeg_dir.join("ffprobe");
        if path.exists() {
            return Some(path);
        }
        return Some(PathBuf::from("ffprobe"));
    }

    None
}
```

Replace `Self::get_ffprobe()` calls in `screenshot_dialog.rs` with `crate::utils::get_ffprobe_path()`.

- [ ] **Step 4: Remove duplicated `hex_to_color()` from `src/ui/tag_manager.rs`**

Delete the `hex_to_color()` method from `src/ui/tag_manager.rs` (lines 191-201).

Replace all `self.hex_to_color(...)` calls with `crate::utils::hex_to_color(...)`.

- [ ] **Step 5: Remove duplicated `get_ffmpeg()` from `src/scanner/mod.rs`**

Delete the `get_ffmpeg()` method from `src/scanner/mod.rs` (lines 106-126).

Replace `Self::get_ffmpeg()` calls with `crate::utils::get_ffmpeg_path()`.

- [ ] **Step 7: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 8: Commit**

```bash
git add src/utils.rs src/ui/main_window.rs src/ui/context_menu.rs src/ui/screenshot_dialog.rs src/ui/tag_manager.rs src/scanner/mod.rs
git commit -m "refactor: replace duplicated utility functions with shared utils module"
```

---

### Task 3: Enable SQLite WAL Mode + Remove Dead Code

**Files:**
- Modify: `src/db/mod.rs`
- Modify: `src/ui/mod.rs`
- Delete: `src/ui/tab_bar.rs`
- Delete: `src/ui/thumbnail_grid.rs`
- Delete: `src/ui/add_path_dialog.rs`

- [ ] **Step 1: Enable WAL mode in `src/db/mod.rs`**

In the `Database::new()` function, add `PRAGMA journal_mode = WAL;` to the `execute_batch` call. Change:

```rust
conn.execute_batch("PRAGMA foreign_keys = ON;")?;
```

To:

```rust
conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
```

- [ ] **Step 2: Remove dead module declarations from `src/ui/mod.rs`**

Change `src/ui/mod.rs` from:

```rust
mod add_path_dialog;
mod context_menu;
mod main_window;
mod screenshot_dialog;
mod settings_dialog;
mod tag_manager;
mod thumbnail_grid;

pub use context_menu::ContextMenu;
pub use main_window::MainWindow;
pub use screenshot_dialog::ScreenshotDialog;
pub use settings_dialog::SettingsDialog;
pub use tag_manager::TagManager;
pub use thumbnail_grid::ThumbnailGrid;
```

To:

```rust
mod context_menu;
mod main_window;
mod screenshot_dialog;
mod settings_dialog;
mod tag_manager;

pub use context_menu::ContextMenu;
pub use main_window::MainWindow;
pub use screenshot_dialog::ScreenshotDialog;
pub use settings_dialog::SettingsDialog;
pub use tag_manager::TagManager;
```

- [ ] **Step 3: Delete unused files**

```bash
git rm src/ui/tab_bar.rs src/ui/thumbnail_grid.rs src/ui/add_path_dialog.rs
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add src/db/mod.rs src/ui/mod.rs
git commit -m "perf: enable SQLite WAL mode and remove dead code (tab_bar, thumbnail_grid, add_path_dialog)"
```

---

### Task 4: Separate Thumbnail Generation from Scanning

**Files:**
- Modify: `src/scanner/mod.rs`
- Modify: `src/app.rs`
- Modify: `src/ui/main_window.rs`

- [ ] **Step 1: Refactor `Scanner::scan_path()` to not generate thumbnails**

Replace the entire `scan_path` method and `create_file_entry` method in `src/scanner/mod.rs` with:

```rust
use crate::db::{Database, FileEntry};
use chrono::{DateTime, Utc};
use log::{error, info};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use walkdir::WalkDir;

pub struct Scanner {
    db: Arc<Mutex<Database>>,
    extensions: Vec<String>,
}

/// Result of a scan operation
pub struct ScanFileResult {
    pub count: usize,
    pub files_needing_thumbnails: Vec<String>,
}

impl Scanner {
    pub fn new(db: Arc<Mutex<Database>>, extensions: Vec<String>) -> Self {
        Self { db, extensions }
    }

    pub async fn scan_path(&self, path_id: i64, root_path: &str) -> Result<ScanFileResult, String> {
        info!("Starting scan for path_id={}, root={}", path_id, root_path);
        let path = PathBuf::from(root_path);

        if !path.exists() {
            return Err(format!("Path does not exist: {}", root_path));
        }

        let extensions = self.extensions.clone();
        let root_str = root_path.to_string();

        // Run blocking I/O on spawn_blocking
        let files = tokio::task::spawn_blocking(move || {
            Self::blocking_scan(&root_str, path_id, &extensions)
        })
        .await
        .map_err(|e| format!("task join error: {e}"))?;

        // Insert files into database
        let mut count = 0;
        let mut files_needing_thumbnails = Vec::new();
        let data_dir = crate::utils::get_data_dir();
        let thumbs_dir = data_dir.join("thumbs");

        for file_entry in files {
            let thumb_path = thumbs_dir.join(format!("{:x}.jpg", md5::compute(file_entry.file_path.as_bytes())));
            let needs_thumb = !thumb_path.exists();
            if needs_thumb {
                files_needing_thumbnails.push(file_entry.file_path.clone());
            }

            let db = self.db.lock().await;
            if let Err(e) = db.upsert_file(&file_entry) {
                error!("Failed to insert file: {:?}", e);
            } else {
                count += 1;
            }
        }

        info!("Scan complete: {} files found, {} need thumbnails", count, files_needing_thumbnails.len());
        Ok(ScanFileResult { count, files_needing_thumbnails })
    }

    fn blocking_scan(path: &str, path_id: i64, extensions: &[String]) -> Vec<FileEntry> {
        let root = PathBuf::from(path);
        let mut entries = Vec::new();

        for entry in WalkDir::new(&root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }

            if let Some(ext) = file_path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if !extensions.contains(&ext_lower) {
                    continue;
                }

                if let Some(file_entry) = Self::create_file_entry(path_id, file_path) {
                    entries.push(file_entry);
                }
            }
        }

        entries
    }

    fn create_file_entry(path_id: i64, file_path: &Path) -> Option<FileEntry> {
        let metadata = file_path.metadata().ok()?;
        let modified_at: DateTime<Utc> = metadata.modified().ok()?.into();

        let file_name = file_path
            .file_name()?
            .to_string_lossy()
            .to_string();

        let file_ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        Some(FileEntry {
            id: 0,
            path_id,
            file_path: file_path.to_string_lossy().to_string(),
            file_name,
            file_ext,
            thumbnail_path: None,
            file_size: metadata.len() as i64,
            modified_at: modified_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            rating: 0,
            has_custom_thumb: false,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }
}
```

- [ ] **Step 2: Add thumbnail progress fields to `App` struct**

In `src/app.rs`, add these fields to the `App` struct (after `scan_complete_rx`):

```rust
pub thumbnail_progress: Option<(usize, usize)>, // (completed, total)
```

- [ ] **Step 3: Update `ScanResult` to include thumbnail queue**

In `src/app.rs`, change `ScanResult` from:

```rust
pub struct ScanResult {
    pub path_id: i64,
    pub count: usize,
    pub success: bool,
    pub message: String,
}
```

To:

```rust
pub struct ScanResult {
    pub path_id: i64,
    pub count: usize,
    pub success: bool,
    pub message: String,
    pub files_needing_thumbnails: Vec<String>,
}
```

- [ ] **Step 4: Update `start_scan` in `src/ui/main_window.rs`**

Replace the `start_scan` function in `src/ui/main_window.rs` with:

```rust
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
```

- [ ] **Step 5: Update scan completion handler in `App::update_impl()`**

In `src/app.rs`, update the scan completion handler in `update_impl()`. Replace:

```rust
if let Some(rx) = self.scan_complete_rx.take() {
    if let Ok(result) = rx.try_recv() {
        self.scanning = false;
        self.scan_status = if result.success {
            format!("完成！找到 {} 個檔案", result.count)
        } else {
            format!("失敗: {}", result.message)
        };
        ctx.request_repaint();
    } else {
        self.scan_complete_rx = Some(rx);
    }
}
```

With:

```rust
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
        ctx.request_repaint();
    } else {
        self.scan_complete_rx = Some(rx);
    }
}
```

- [ ] **Step 6: Add `start_thumbnail_generation` method to `App`**

Add this method to `impl App` in `src/app.rs`:

```rust
pub fn start_thumbnail_generation(&mut self, files: Vec<String>, path_id: i64) {
    let total = files.len();
    let db = self.db.clone();
    let data_dir = crate::utils::get_data_dir();
    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();

    self.thumbnail_progress = Some((0, total));
    self.scan_status = format!("生成縮圖中... 0/{}", total);

    // Store channels for polling
    self.thumb_progress_rx = Some(progress_rx);
    self.thumb_done_rx = Some(done_rx);

    std::thread::spawn(move || {
        if let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() {
            let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path);
            for (i, file_path) in files.iter().enumerate() {
                match gen.generate_thumbnail_at_frame(file_path, 300) {
                    Ok(thumb_path) => {
                        let db_guard = db.blocking_lock();
                        // Find file by path to get its ID
                        if let Ok(all_files) = db_guard.get_files_for_path(path_id) {
                            if let Some(file) = all_files.iter().find(|f| f.file_path == *file_path) {
                                let _ = db_guard.update_thumbnail(file.id, &thumb_path);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to generate thumbnail for {}: {}", file_path, e);
                    }
                }
                let _ = progress_tx.send(i + 1);
            }
        }
        let _ = done_tx.send(path_id);
    });
}
```

- [ ] **Step 7: Add thumbnail progress polling fields to `App`**

Add these fields to the `App` struct in `src/app.rs`:

```rust
pub thumb_progress_rx: Option<std::sync::mpsc::Receiver<usize>>,
pub thumb_done_rx: Option<std::sync::mpsc::Receiver<i64>>,
```

- [ ] **Step 8: Add thumbnail progress polling to `update_impl()`**

Add this code to `update_impl()` in `src/app.rs`, after the scan completion handler:

```rust
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
```

- [ ] **Step 9: Initialize new fields in `App::new()`**

Add these initializations to the `Self { ... }` block in `App::new()`:

```rust
thumbnail_progress: None,
thumb_progress_rx: None,
thumb_done_rx: None,
```

- [ ] **Step 10: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 11: Commit**

```bash
git add src/scanner/mod.rs src/app.rs src/ui/main_window.rs
git commit -m "perf: separate thumbnail generation from scanning into background thread with progress reporting"
```

---

### Task 5: Lazy-Load Files Per Tab + Cache Results

**Files:**
- Modify: `src/app.rs`
- Modify: `src/ui/main_window.rs`

- [ ] **Step 1: Add cache invalidation flag to `App`**

Add this field to the `App` struct in `src/app.rs`:

```rust
pub cache_invalidated: bool,
```

Initialize it to `false` in `App::new()`.

- [ ] **Step 2: Refactor `get_current_files()` to use cache**

Replace `get_current_files()` in `src/app.rs` with:

```rust
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
```

- [ ] **Step 3: Add `invalidate_cache()` method**

Add to `impl App` in `src/app.rs`:

```rust
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
```

- [ ] **Step 4: Call `refresh_cache_if_needed()` in `update_impl()`**

Add this line to `update_impl()` in `src/app.rs`, before `MainWindow::show_main(ctx, self);`:

```rust
self.refresh_cache_if_needed();
```

- [ ] **Step 5: Invalidate cache on filter/sort changes**

In `src/ui/main_window.rs`, add `app.invalidate_cache()` after each filter/sort change:

After `app.sort_by = SortBy::Name;` (line 44):
```rust
app.invalidate_cache();
```

After `app.sort_by = SortBy::Date;` (line 48):
```rust
app.invalidate_cache();
```

After `app.sort_by = SortBy::Rating;` (line 52):
```rust
app.invalidate_cache();
```

After `app.rating_filter = None;` (line 66):
```rust
app.invalidate_cache();
```

After `app.rating_filter = Some(i);` (line 72):
```rust
app.invalidate_cache();
```

After `app.tag_filter = None;` (line 83):
```rust
app.invalidate_cache();
```

After `app.tag_filter = Some(tag.id);` (line 88):
```rust
app.invalidate_cache();
```

- [ ] **Step 6: Initialize `cache_invalidated` in `App::new()`**

Add `cache_invalidated: false,` to the `Self { ... }` block.

- [ ] **Step 7: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/ui/main_window.rs
git commit -m "perf: lazy-load files per tab with cache invalidation on filter changes"
```

---

### Task 6: Integration Test + Manual Verification

**Files:**
- No file changes

- [ ] **Step 1: Build release**

Run: `cargo build --release`
Expected: Successful build

- [ ] **Step 2: Manual test — scan performance**

1. Run the app
2. Add a folder with 50+ video files
3. Click "🔄 更新資料庫"
4. Verify: UI remains responsive (can scroll, click buttons)
5. Verify: Scan completes quickly (no thumbnail generation during scan)
6. Verify: "生成縮圖中..." progress appears after scan
7. Verify: Thumbnails appear progressively

- [ ] **Step 3: Manual test — lazy loading**

1. Add a second watched path
2. Switch between tabs
3. Verify: Files load only when tab is selected
4. Verify: Switching back to a loaded tab shows cached files instantly

- [ ] **Step 4: Manual test — filter/sort cache**

1. Apply a search filter
2. Change sort order
3. Verify: Results update correctly without DB re-read

- [ ] **Step 5: Manual test — WAL mode**

Run: `sqlite3 %LOCALAPPDATA%\video-thumb-viewer\data.db "PRAGMA journal_mode;"`
Expected: `wal`
