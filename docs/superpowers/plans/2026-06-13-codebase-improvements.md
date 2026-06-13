# Codebase Improvement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address 21 issues from codebase review across three phases: critical fixes, medium improvements, and minor cleanup.

**Architecture:** Incremental refactoring of a Rust/egui desktop app. Phase 1 fixes reliability and performance issues. Phase 2 improves code quality and UX. Phase 3 cleans up dead code and adds CI.

**Tech Stack:** Rust, egui/eframe, rusqlite, tokio, chrono, lru, tempfile, once_cell

**Spec:** `docs/superpowers/specs/2026-06-13-codebase-improvement-design.md`

---

## File Structure

**Modified files:**
- `Cargo.toml` - Add dependencies (lru, tempfile, once_cell)
- `src/main.rs` - Initialize config cache
- `src/utils.rs` - Add config caching with OnceLock
- `src/app.rs` - Mutex handling, async file loading, texture LRU, window state, scan cancellation, search debouncing
- `src/config.rs` - Window state persistence
- `src/db/mod.rs` - Add safe_lock helper
- `src/db/models.rs` - Date types, dead code removal
- `src/db/queries.rs` - Transaction safety, date parsing, dead code removal
- `src/scanner/mod.rs` - Scan cancellation flag
- `src/thumbnail/mod.rs` - Remove duplicate play_video
- `src/lang.rs` - Refactor to HashMap lookup
- `src/ui/main_window.rs` - Async cleanup, search debouncing, Rc<RefCell> cleanup
- `src/ui/context_menu.rs` - Dead code removal
- `src/ui/screenshot_dialog.rs` - Temp file cleanup
- `src/ui/tag_manager.rs` - Color preset buttons
- `src/ui/settings_dialog.rs` - Config cache reload
- `build.rs` - Windows conditional compilation

**Created files:**
- `.github/workflows/ci.yml` - CI/CD workflow

---

## Phase 1: Critical/Important Fixes

### Task 1: Config Caching

**Files:**
- Modify: `src/utils.rs:1-10`
- Modify: `src/main.rs:38-44`
- Modify: `src/ui/settings_dialog.rs:161-167`

- [ ] **Step 1: Add OnceLock config cache to utils.rs**

Replace the entire `get_data_dir()` function with a cached version:

```rust
use std::path::PathBuf;
use std::sync::OnceLock;
use crate::config::Config;

static CONFIG_CACHE: OnceLock<Config> = OnceLock::new();

pub fn init_config() {
    let config = Config::load();
    CONFIG_CACHE.set(config).ok();
}

pub fn reload_config() {
    let config = Config::load();
    // OnceLock can only be set once, so we use unsafe to replace
    // This is safe because we control access and it's only called from UI thread
    unsafe {
        let ptr = &CONFIG_CACHE as *const OnceLock<Config> as *mut OnceLock<Config>;
        // Take the old value out by creating a new OnceLock
        // Actually, OnceLock doesn't support replacement. Use a different approach.
    }
}
```

Actually, `OnceLock` doesn't support replacement. Use `std::sync::RwLock` instead:

```rust
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
use crate::config::Config;

static CONFIG_CACHE: OnceLock<RwLock<Config>> = OnceLock::new();

pub fn init_config() {
    let config = Config::load();
    CONFIG_CACHE.set(RwLock::new(config)).ok();
}

pub fn reload_config() {
    if let Some(cache) = CONFIG_CACHE.get() {
        let config = Config::load();
        if let Ok(mut guard) = cache.write() {
            *guard = config;
        }
    }
}

pub fn get_data_dir() -> PathBuf {
    if let Some(cache) = CONFIG_CACHE.get() {
        if let Ok(guard) = cache.read() {
            return guard.get_data_dir();
        }
    }
    // Fallback if not initialized
    Config::default().get_data_dir()
}
```

- [ ] **Step 2: Initialize config cache in main.rs**

In `src/main.rs`, after the logger initialization (line 42), add:

```rust
utils::init_config();
```

- [ ] **Step 3: Reload config after settings save**

In `src/ui/settings_dialog.rs`, at the end of the `save()` method (after `config.save()`), add:

```rust
crate::utils::reload_config();
```

- [ ] **Step 4: Verify build**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add src/utils.rs src/main.rs src/ui/settings_dialog.rs
git commit -m "perf: cache config to avoid repeated disk reads"
```

---

### Task 2: Mutex Poisoning Handling

**Files:**
- Modify: `src/db/mod.rs:10-12` (add helper method)
- Modify: `src/app.rs` (all `.lock().unwrap()` calls)
- Modify: `src/ui/*.rs` (all `.lock().unwrap()` calls)
- Modify: `src/scanner/mod.rs` (DB lock calls)
- Modify: `src/thumbnail/mod.rs` (DB lock calls)

- [ ] **Step 1: Add safe_lock helper to Database**

In `src/db/mod.rs`, add a method to `App` struct or create a helper. Since `Database` is wrapped in `Arc<Mutex<Database>>`, add a trait extension:

In `src/db/mod.rs`, after the `Database` struct definition:

```rust
pub trait SafeMutexExt<T> {
    fn safe_lock(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> SafeMutexExt<T> for std::sync::Mutex<T> {
    fn safe_lock(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}
```

- [ ] **Step 2: Replace all `.lock().unwrap()` calls**

Search and replace all occurrences of `.lock().unwrap()` with `.safe_lock()` across:
- `src/app.rs`
- `src/ui/main_window.rs`
- `src/ui/context_menu.rs`
- `src/ui/screenshot_dialog.rs`
- `src/ui/tag_manager.rs`
- `src/ui/settings_dialog.rs`
- `src/scanner/mod.rs`
- `src/thumbnail/mod.rs`

Use find-and-replace: `.lock().unwrap()` → `.safe_lock()`

Add import at top of each file:
```rust
use crate::db::SafeMutexExt;
```

- [ ] **Step 3: Verify build and tests**

Run: `cargo test`
Expected: All 38 tests pass

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix: handle mutex poisoning to prevent app crashes"
```

---

### Task 3: Transaction Safety

**Files:**
- Modify: `src/db/queries.rs:19`

- [ ] **Step 1: Replace unchecked_transaction with transaction**

In `src/db/queries.rs`, line 19, change:

```rust
let tx = self.conn.unchecked_transaction()?;
```

to:

```rust
let tx = self.conn.transaction()?;
```

- [ ] **Step 2: Verify tests**

Run: `cargo test db::queries::tests::test_remove_path`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/db/queries.rs
git commit -m "fix: use safe transaction in remove_path"
```

---

### Task 4: Deduplicate play_video

**Files:**
- Modify: `src/thumbnail/mod.rs:262-296` (remove method)

- [ ] **Step 1: Remove duplicate play_video from ThumbnailGenerator**

In `src/thumbnail/mod.rs`, delete the entire `play_video` method (lines 262-296).

- [ ] **Step 2: Verify build**

Run: `cargo check`
Expected: Compiles without errors (the method was unused)

- [ ] **Step 3: Commit**

```bash
git add src/thumbnail/mod.rs
git commit -m "refactor: remove duplicate play_video method"
```

---

### Task 5: Window State Persistence

**Files:**
- Modify: `src/app.rs:552-560` (implement on_exit)

- [ ] **Step 1: Implement on_exit to save window state**

In `src/app.rs`, update the `on_exit` method in the `eframe::App` impl:

```rust
fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
    // Save window state
    let mut config = crate::config::Config::load();
    // Note: We can't get the actual window rect from eframe in on_exit
    // So we'll save the initial config's window state
    // For now, just save the config to persist any changes
    config.save().ok();
}
```

Actually, eframe doesn't provide window rect in `on_exit`. We need to capture it during `update`. Add a field to `App`:

```rust
pub last_window_rect: Option<egui::Rect>,
```

In `update_impl`, capture the rect:

```rust
self.last_window_rect = Some(ctx.screen_rect());
```

Then in `on_exit`:

```rust
fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
    if let Some(rect) = self.last_window_rect {
        let mut config = crate::config::Config::load();
        config.set_window_state(crate::config::WindowState {
            width: rect.width(),
            height: rect.height(),
            x: Some(rect.min.x),
            y: Some(rect.min.y),
        });
        config.save().ok();
    }
}
```

- [ ] **Step 2: Add last_window_rect field to App struct**

In `src/app.rs`, add to the `App` struct:

```rust
pub last_window_rect: Option<egui::Rect>,
```

Initialize in `App::new`:

```rust
last_window_rect: None,
```

- [ ] **Step 3: Capture window rect in update_impl**

In `update_impl`, near the top (after theme loading), add:

```rust
self.last_window_rect = Some(ctx.screen_rect());
```

- [ ] **Step 4: Verify build**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: persist window position and size across restarts"
```

---

### Task 6: Texture Cache LRU Eviction

**Files:**
- Modify: `Cargo.toml:8-35` (add lru dependency)
- Modify: `src/app.rs:55` (change texture_cache type)
- Modify: `src/ui/main_window.rs:481-494` (update load_tex)

- [ ] **Step 1: Add lru dependency to Cargo.toml**

In `Cargo.toml`, add to `[dependencies]`:

```toml
lru = "0.12"
```

- [ ] **Step 2: Change texture_cache type in App struct**

In `src/app.rs`, change the `texture_cache` field type:

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

// In App struct:
pub texture_cache: LruCache<String, egui::TextureHandle>,
```

Initialize in `App::new`:

```rust
texture_cache: LruCache::new(NonZeroUsize::new(200).unwrap()),
```

Update `clear_texture_cache`:

```rust
pub fn clear_texture_cache(&mut self) {
    self.texture_cache.clear();
}
```

- [ ] **Step 3: Update load_tex in main_window.rs**

In `src/ui/main_window.rs`, update `load_tex`:

```rust
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
```

- [ ] **Step 4: Verify build and tests**

Run: `cargo test`
Expected: All 38 tests pass

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/app.rs src/ui/main_window.rs
git commit -m "perf: add LRU eviction to texture cache (200 entry limit)"
```

---

### Task 7: Scan Cancellation

**Files:**
- Modify: `src/scanner/mod.rs:9-12,105-132` (add cancel flag)
- Modify: `src/app.rs:25-33` (add cancel flag field)
- Modify: `src/ui/main_window.rs:545-598` (set cancel flag before new scan)

- [ ] **Step 1: Add cancel flag to Scanner**

In `src/scanner/mod.rs`, update `Scanner` struct:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Scanner {
    db: Arc<Mutex<Database>>,
    extensions: Vec<String>,
    cancel_flag: Arc<AtomicBool>,
}
```

Update `Scanner::new`:

```rust
pub fn new(db: Arc<Mutex<Database>>, extensions: Vec<String>, cancel_flag: Arc<AtomicBool>) -> Self {
    Self { db, extensions, cancel_flag }
}
```

- [ ] **Step 2: Check cancel flag in blocking_scan**

In `blocking_scan`, add cancel check in the loop:

```rust
for (i, entry) in WalkDir::new(&root)
    .follow_links(true)
    .into_iter()
    .filter_map(|e| e.ok())
    .enumerate()
{
    if i % 100 == 0 && cancel_flag.load(Ordering::Relaxed) {
        log::info!("Scan cancelled");
        return entries;
    }
    
    let file_path = entry.path();
    // ... rest of loop
}
```

Update `scan_path` to pass cancel_flag:

```rust
let cancel_flag = self.cancel_flag.clone();
let files = tokio::task::spawn_blocking(move || {
    Self::blocking_scan(&root_str, path_id, &extensions, cancel_flag)
})
```

- [ ] **Step 3: Add cancel flag to App struct**

In `src/app.rs`, add to `App` struct:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

pub scan_cancel_flag: Arc<AtomicBool>,
```

Initialize in `App::new`:

```rust
scan_cancel_flag: Arc::new(AtomicBool::new(false)),
```

- [ ] **Step 4: Set cancel flag before starting new scan**

In `src/ui/main_window.rs`, in `start_scan`, before spawning the thread:

```rust
app.scan_cancel_flag.store(true, Ordering::Relaxed);
app.scan_cancel_flag = Arc::new(AtomicBool::new(false));
```

Pass the flag to Scanner:

```rust
let cancel_flag = app.scan_cancel_flag.clone();
let scanner = Scanner::new(db.clone(), extensions, cancel_flag);
```

- [ ] **Step 5: Verify build and tests**

Run: `cargo test`
Expected: All 38 tests pass

- [ ] **Step 6: Commit**

```bash
git add src/scanner/mod.rs src/app.rs src/ui/main_window.rs
git commit -m "feat: add scan cancellation support"
```

---

### Task 8: UI Thread Blocking - load_files_for_path

**Files:**
- Modify: `src/app.rs:146-176` (refactor to async)
- Modify: `src/app.rs:8-56` (add channel fields)
- Modify: `src/app.rs:431-505` (poll channels in update_impl)

- [ ] **Step 1: Add channel fields to App struct**

In `src/app.rs`, add to `App` struct:

```rust
pub files_load_rx: Option<std::sync::mpsc::Receiver<(i64, Vec<FileEntry>, std::collections::HashMap<i64, Vec<i64>>)>>,
```

Initialize in `App::new`:

```rust
files_load_rx: None,
```

- [ ] **Step 2: Refactor load_files_for_path to async**

Replace `load_files_for_path`:

```rust
pub fn load_files_for_path(&mut self, path_id: i64) {
    self.texture_cache.clear();
    self.loading_files = true;
    
    let db = self.db.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    self.files_load_rx = Some(rx);
    
    std::thread::spawn(move || {
        let db_guard = db.safe_lock();
        let files = db_guard.get_files_for_path(path_id).unwrap_or_default();
        let tag_map = db_guard.get_all_file_tags(path_id).unwrap_or_default();
        drop(db_guard);
        
        let _ = tx.send((path_id, files, tag_map));
    });
}
```

- [ ] **Step 3: Poll channel in update_impl**

In `update_impl`, after the thumbnail progress polling, add:

```rust
if let Some(rx) = self.files_load_rx.as_ref() {
    if let Ok((path_id, files, tag_map)) = rx.try_recv() {
        self.files_load_rx = None;
        self.loading_files = false;
        
        // Clear stale tag map entries
        if let Some(old_files) = self.files.get(&path_id) {
            for file in old_files.iter() {
                self.file_tag_map.remove(&file.id);
            }
        }
        
        self.file_tag_map.extend(tag_map);
        self.files.insert(path_id, Arc::new(files));
        self.filter_cache = None;
        
        ctx.request_repaint();
    }
}
```

- [ ] **Step 4: Verify build and tests**

Run: `cargo test`
Expected: All 38 tests pass

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "perf: make load_files_for_path non-blocking"
```

---

### Task 9: UI Thread Blocking - clean_unused_thumbnails

**Files:**
- Modify: `src/app.rs:8-56` (add channel fields)
- Modify: `src/ui/main_window.rs:496-543` (refactor to background thread)
- Modify: `src/app.rs:431-505` (poll channel in update_impl)

- [ ] **Step 1: Add channel field to App struct**

In `src/app.rs`, add to `App` struct:

```rust
pub clean_thumbs_rx: Option<std::sync::mpsc::Receiver<String>>,
```

Initialize in `App::new`:

```rust
clean_thumbs_rx: None,
```

- [ ] **Step 2: Refactor clean_unused_thumbnails to background thread**

In `src/ui/main_window.rs`, refactor `clean_unused_thumbnails`:

```rust
fn clean_unused_thumbnails(app: &mut App) {
    let lang = app.lang;
    let data_dir = crate::utils::get_data_dir();
    let thumbs_dir = data_dir.join("thumbs");
    
    if !thumbs_dir.exists() {
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
}
```

- [ ] **Step 3: Poll channel in update_impl**

In `update_impl`, add:

```rust
if let Some(rx) = self.clean_thumbs_rx.as_ref() {
    if let Ok(msg) = rx.try_recv() {
        self.clean_thumbs_rx = None;
        self.scanning = false;
        self.scan_status = msg;
        ctx.request_repaint();
    }
}
```

- [ ] **Step 4: Verify build and tests**

Run: `cargo test`
Expected: All 38 tests pass

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/ui/main_window.rs
git commit -m "perf: make clean_unused_thumbnails non-blocking"
```

---

## Phase 2: Medium Issues

### Task 10: Search Debouncing

**Files:**
- Modify: `src/app.rs:8-56` (add field)
- Modify: `src/ui/main_window.rs:67` (add debounce check)

- [ ] **Step 1: Add search_last_changed field to App**

In `src/app.rs`, add to `App` struct:

```rust
pub search_last_changed: std::time::Instant,
```

Initialize in `App::new`:

```rust
search_last_changed: std::time::Instant::now(),
```

- [ ] **Step 2: Add debounce logic in main_window.rs**

In `src/ui/main_window.rs`, after the search text edit (line 67), add:

```rust
let prev_query = app.search_query.clone();
ui.text_edit_singleline(&mut app.search_query);
if app.search_query != prev_query {
    app.search_last_changed = std::time::Instant::now();
}

// Only invalidate if debounce period has passed
if app.search_last_changed.elapsed().as_millis() >= 200 {
    app.invalidate_cache();
}
```

- [ ] **Step 3: Verify build**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/ui/main_window.rs
git commit -m "perf: add 200ms debounce to search input"
```

---

### Task 11: Screenshot Temp File Cleanup

**Files:**
- Modify: `Cargo.toml:8-35` (add tempfile dependency)
- Modify: `src/ui/screenshot_dialog.rs:222-266` (use tempfile)

- [ ] **Step 1: Add tempfile dependency**

In `Cargo.toml`, add to `[dependencies]`:

```toml
tempfile = "3.10"
```

- [ ] **Step 2: Use tempfile in screenshot_dialog.rs**

In `src/ui/screenshot_dialog.rs`, in `update_preview`, replace the temp file logic:

```rust
std::thread::spawn(move || {
    let temp_file = tempfile::NamedTempFile::new_in(&data_dir).ok();
    let temp_path = match &temp_file {
        Some(f) => f.path().to_path_buf(),
        None => {
            let _ = tx.send(Err("Failed to create temp file".to_string()));
            return;
        }
    };
    
    // ... ffmpeg command using temp_path ...
    
    // temp_file is automatically deleted when dropped
});
```

- [ ] **Step 3: Verify build**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/ui/screenshot_dialog.rs
git commit -m "fix: use tempfile for screenshot previews to prevent orphaned files"
```

---

### Task 12: Color Preset Click Detection

**Files:**
- Modify: `src/ui/tag_manager.rs:127-148` (refactor show_color_presets)

- [ ] **Step 1: Refactor show_color_presets to use Button widgets**

Replace `show_color_presets`:

```rust
fn show_color_presets(&mut self, ui: &mut egui::Ui, is_edit: bool) {
    ui.horizontal_wrapped(|ui| {
        for color in PRESET_COLORS {
            let c = crate::utils::hex_to_color(color);
            let button = egui::Button::new("")
                .fill(c)
                .min_size([20.0, 20.0].into());
            
            if ui.add(button).clicked() {
                if is_edit {
                    self.edit_color = color.to_string();
                } else {
                    self.new_tag_color = color.to_string();
                }
            }
        }
    });
}
```

- [ ] **Step 2: Verify build**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 3: Commit**

```bash
git add src/ui/tag_manager.rs
git commit -m "refactor: use Button widgets for color presets"
```

---

### Task 13: Rc<RefCell> Cleanup

**Files:**
- Modify: `src/ui/main_window.rs:366-479` (refactor show_thumbnail_item)

- [ ] **Step 1: Refactor show_thumbnail_item to avoid Rc<RefCell>**

Restructure the function to avoid needing thumb_rect outside the closure. The key is to handle all click logic inside the `allocate_ui` closure:

```rust
fn show_thumbnail_item(ui: &mut egui::Ui, file: &crate::db::FileEntry, size: f32, ctx: &egui::Context, app: &mut App) {
    let response = ui.allocate_ui([size, size + 50.0].into(), |ui| {
        ui.vertical(|ui| {
            let thumb_size_inner = size - 4.0;
            let thumb_rect = ui.allocate_rect(
                egui::Rect::from_min_size(ui.cursor().min, [thumb_size_inner, thumb_size_inner].into()),
                egui::Sense::click_and_drag(),
            );
            
            // ... drawing code ...
            
            // Handle clicks inside the closure
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
                } else if let Some(ref mut menu) = app.context_menu {
                    menu.file = Some(file.clone());
                    menu.click_pos = thumb_rect.interact_pointer_pos()
                        .unwrap_or_else(|| thumb_rect.rect.min);
                }
            }
            
            // ... rest of drawing code ...
        });
    });
}
```

- [ ] **Step 2: Verify build and tests**

Run: `cargo test`
Expected: All 38 tests pass

- [ ] **Step 3: Commit**

```bash
git add src/ui/main_window.rs
git commit -m "refactor: remove Rc<RefCell> workaround in thumbnail rendering"
```

---

### Task 14: i18n Refactor

**Files:**
- Modify: `Cargo.toml:8-35` (add once_cell dependency)
- Modify: `src/lang.rs:34-318` (refactor to HashMap)

- [ ] **Step 1: Add once_cell dependency**

In `Cargo.toml`, add to `[dependencies]`:

```toml
once_cell = "1.19"
```

- [ ] **Step 2: Refactor lang.rs to use HashMap**

Replace the `t` function with HashMap lookup:

```rust
use once_cell::sync::Lazy;
use std::collections::HashMap;

static TRANSLATIONS: Lazy<HashMap<&'static str, HashMap<Lang, &'static str>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    m.insert("add_path", [(Lang::ZhTw, "+ 新增路徑"), (Lang::En, "+ Add Path")].into_iter().collect());
    m.insert("scanning_prefix", [(Lang::ZhTw, "掃描中... "), (Lang::En, "Scanning... ")].into_iter().collect());
    // ... all other translations ...
    
    m
});

pub fn t(lang: Lang, key: &str) -> &'static str {
    TRANSLATIONS
        .get(key)
        .and_then(|map| map.get(&lang))
        .or_else(|| TRANSLATIONS.get(key).and_then(|map| map.get(&Lang::En)))
        .copied()
        .unwrap_or("")
}
```

- [ ] **Step 3: Verify build**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/lang.rs
git commit -m "refactor: use HashMap for i18n translations"
```

---

### Task 15: Arc Clone Optimization

**Files:**
- Modify: `src/app.rs:178-243` (optimize get_current_files)

- [ ] **Step 1: Optimize get_current_files to avoid unnecessary clones**

Refactor `get_current_files` to use `Arc::make_mut` for copy-on-write:

```rust
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
        let db_guard = self.db.safe_lock();
        let files = db_guard.get_files_for_path(path_id).unwrap_or_default();
        let files = Arc::new(files);
        self.files.insert(path_id, Arc::clone(&files));
        files
    };
    
    // Use Arc::make_mut for copy-on-write
    let mut files = Arc::try_unwrap(base_files).unwrap_or_else(|arc| (*arc).clone());
    
    // Apply filters
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
```

- [ ] **Step 2: Verify build and tests**

Run: `cargo test`
Expected: All 38 tests pass

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "perf: optimize get_current_files with Arc::try_unwrap"
```

---

### Task 16: Date Type Safety

**Files:**
- Modify: `src/db/models.rs:1-51` (change date types)
- Modify: `src/db/queries.rs` (parse/format dates)
- Modify: `src/scanner/mod.rs:134-161` (use DateTime)

- [ ] **Step 1: Change date types in models**

In `src/db/models.rs`, update structs:

```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct WatchedPath {
    pub id: i64,
    pub path: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub id: i64,
    pub path_id: i64,
    pub file_path: String,
    pub file_name: String,
    pub file_ext: String,
    pub thumbnail_path: Option<String>,
    pub file_size: i64,
    pub modified_at: DateTime<Utc>,
    pub rating: i32,
    pub has_custom_thumb: bool,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update queries to parse/format dates**

In `src/db/queries.rs`, update `get_files_for_path` and other queries to parse dates:

```rust
let created_at_str: String = row.get(10)?;
let created_at = DateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
    .map(|dt| dt.with_timezone(&Utc))
    .unwrap_or_else(|_| Utc::now());
```

- [ ] **Step 3: Update scanner to use DateTime**

In `src/scanner/mod.rs`, update `create_file_entry`:

```rust
let modified_at: DateTime<Utc> = metadata.modified().ok()?.into();
// ... 
modified_at,
created_at: Utc::now(),
```

- [ ] **Step 4: Verify build and tests**

Run: `cargo test`
Expected: All 38 tests pass (may need to update test fixtures)

- [ ] **Step 5: Commit**

```bash
git add src/db/models.rs src/db/queries.rs src/scanner/mod.rs
git commit -m "refactor: use chrono::DateTime for date fields"
```

---

## Phase 3: Minor Cleanup

### Task 17: Dead Code Cleanup

**Files:**
- Modify: `src/db/models.rs` (remove unused items)
- Modify: `src/db/queries.rs` (remove unused methods)
- Modify: `src/scanner/mod.rs` (remove unused function)
- Modify: `src/thumbnail/mod.rs` (already done in Task 4)
- Modify: `src/ui/context_menu.rs` (remove unused fields/methods)
- Modify: `src/app.rs` (remove unused field)

- [ ] **Step 1: Remove unused items from db/models.rs**

Remove:
- `Tag::new` method
- `FileWithTags` struct

Add `#[allow(dead_code)]` to `created_at` fields if needed.

- [ ] **Step 2: Remove unused methods from db/queries.rs**

Remove:
- `upsert_file`
- `get_file_by_id`
- `tag_name_exists`

- [ ] **Step 3: Remove unused function from scanner/mod.rs**

Remove:
- `get_file_duration`

- [ ] **Step 4: Remove unused fields/methods from context_menu.rs**

Remove:
- `ContextMenu::add_tag_name`
- `ContextMenu::time_input`
- `add_tag_internal`
- `capture_screenshot_internal`
- `parse_time_string`

- [ ] **Step 5: Remove unused field from app.rs**

Remove:
- `scan_complete_tx`

- [ ] **Step 6: Verify build**

Run: `cargo check`
Expected: Compiles with no warnings

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: remove dead code and unused methods"
```

---

### Task 18: Indentation Fix

**Files:**
- Modify: `src/app.rs:118`

- [ ] **Step 1: Fix indentation**

Fix the inconsistent indentation at line 118 in `App::new`.

- [ ] **Step 2: Commit**

```bash
git add src/app.rs
git commit -m "style: fix indentation in App::new"
```

---

### Task 19: winresource Conditional Compilation

**Files:**
- Modify: `Cargo.toml:37-39`
- Modify: `build.rs`

- [ ] **Step 1: Move winresource to Windows-specific dependencies**

In `Cargo.toml`, change:

```toml
[build-dependencies]
winresource = "0.1"
```

to:

```toml
[target.'cfg(windows)'.build-dependencies]
winresource = "0.1"
```

- [ ] **Step 2: Add cfg guard to build.rs**

In `build.rs`, wrap the winresource code:

```rust
fn main() {
    #[cfg(windows)]
    {
        // existing winresource code
    }
}
```

- [ ] **Step 3: Verify build**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml build.rs
git commit -m "build: make winresource Windows-only"
```

---

### Task 20: CI/CD Configuration

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create GitHub Actions workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [ master, codebase-improvements ]
  pull_request:
    branches: [ master ]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: windows-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo check --all-features
    
  test:
    runs-on: windows-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test --all-features
    
  clippy:
    runs-on: windows-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy
    - run: cargo clippy -- -D warnings
    
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo build --release
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add GitHub Actions workflow for check, test, clippy, and build"
```

---

## Summary

Total tasks: 20 (Phase 1: 9, Phase 2: 7, Phase 3: 4)

Each task produces a working build with all tests passing. Tasks are ordered by dependency and severity.
