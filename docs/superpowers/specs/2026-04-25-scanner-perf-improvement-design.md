# Scanner Performance Improvement — Async Blocking Operations

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all UI thread blocking during database scanning and thumbnail generation, making the application fully responsive at all times.

**Architecture:** All file system I/O (`WalkDir`, `metadata()`) and all subprocess calls (ffmpeg) are moved off the UI thread using `tokio::task::spawn_blocking`. The scanner module is refactored to use a proper async API rather than spawning a nested tokio runtime. The thumbnail generator gains async wrappers while preserving existing sync APIs for immediate-mode use. A dedicated progress reporting system bridges background workers with the UI state.

**Tech Stack:** Rust, tokio (already in use), walkdir (already in use), ffmpeg subprocess, egui/eframe.

---

## Problem Statement

The application exhibits two distinct freeze patterns:

1. **Initial complete freeze (B2d-iii Phase 1):** When the user clicks "更新資料庫", the entire UI becomes completely unresponsive for an extended period. This is caused by `WalkDir::new(&path).into_iter().filter_map(|e| e.ok())` — a synchronous I/O-bound loop running on a `std::thread::spawn`-created nested tokio runtime that shares the main thread's event capacity poorly. Additionally, `file_path.metadata()` is a blocking syscall.

2. **Thumbnail loading freeze (B2d-iii Phase 2):** After the initial scan completes, the UI partially resumes, but ffmpeg subprocess calls in `context_menu.rs` and `main_window.rs` still block synchronously. Specifically:
   - `ThumbnailGenerator::generate_thumbnail()` calls `Command::new(&self.ffmpeg_path).output().map_err(...)` — this is a blocking wait.
   - `play_video()` spawns a process but the setup above it includes synchronous path lookups.
   - `capture_screenshot_internal()` does a full ffmpeg subprocess wait on what may be the UI thread.

The root cause in both phases is that I/O-bound and subprocess-bound work runs on threads that either block the event loop or compete with UI rendering.

---

## Design

### Module 1: scanner/mod.rs — Proper Async Refactor

**Problem:** `scanner/mod.rs` currently uses `std::thread::spawn` with `tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(scanner.scan_path(...))`. This creates a nested runtime inside a `std::thread`, which is fragile and doesn't properly offload blocking work.

**Solution:** Refactor `ScanResult`, `Scanner`, and the calling code in `main_window.rs`/`app.rs` so that:

1. `Scanner` holds an `Arc<Mutex<Database>>` and a `Vec<String>` of extensions (already does, no change needed).
2. `Scanner::scan_path` becomes a true async method:
   ```rust
   pub async fn scan_path(&self, path_id: i64, root_path: &str) -> Result<usize, String>
   ```
3. Inside `scan_path`, the heavy I/O loop is wrapped in `tokio::task::spawn_blocking`:
   ```rust
   let extensions = self.extensions.clone();
   let root_clone = root_path.to_string();
   let path_id_inner = path_id;

   let count = tokio::task::spawn_blocking(move || {
       Self::blocking_scan(&root_clone, path_id_inner, &extensions)
   })
   .await
   .map_err(|e| format!("task join error: {e}"))??;
   ```
   `spawn_blocking` moves execution to tokio's dedicated blocking thread pool (unlimited size by default, managed by tokio), freeing the async event loop completely.
4. The `blocking_scan` function is a pure synchronous function (no `self` borrow):
   ```rust
   fn blocking_scan(path: &str, path_id: i64, extensions: &[String]) -> Result<usize, String>
   ```
5. After the scan, thumbnail generation for found files is batched: instead of generating thumbnails inline in `blocking_scan`, the function collects all found `FileEntry` structs and inserts them into the database. A separate system (see Module 2) handles thumbnail generation asynchronously.

**Why not batch thumbnails in the same `spawn_blocking` call?** Because ffmpeg subprocesses are the heaviest operations and should be parallelized across multiple blocking threads, not serialized in one call.

### Module 2: thumbnail/mod.rs — Async API Additions

**Problem:** `ThumbnailGenerator` only has synchronous methods. Call sites in `context_menu.rs` and `main_window.rs` cannot use async/await.

**Solution:** Add async wrappers while preserving existing sync APIs for immediate-mode use:

```rust
impl ThumbnailGenerator {
    // Existing sync API (kept for context_menu.rs immediate-mode calls)
    pub fn generate_thumbnail_sync(&self, video_path: &str) -> Result<String, String>

    // New async wrapper
    pub async fn generate_thumbnail(&self, video_path: &str) -> Result<String, String>
    where Self: Sized {
        let video_path = video_path.to_string();
        let thumb_dir = self.get_thumbs_dir();
        let ffmpeg_path = self.ffmpeg_path.clone();
        let width = self.width;
        let height = self.height;

        tokio::task::spawn_blocking(move || {
            Self::blocking_generate_thumbnail(&thumb_dir, &ffmpeg_path, &video_path, width, height)
        })
        .await
        .map_err(|e| format!("spawn_blocking failed: {e}"))?
    }

    pub async fn generate_thumbnail_at_time(&self, video_path: &str, time_seconds: f64) -> Result<String, String>
    where Self: Sized {
        // Same pattern: spawn_blocking -> blocking helper
    }

    pub async fn play_video_async(&self, video_path: &str, player: &str) -> Result<(), String>

    // Pure blocking factory methods (no `self` borrow, safe to call from spawn_blocking)
    fn blocking_generate_thumbnail(thumb_dir: &Path, ffmpeg: &Path, path: &str, w: u32, h: u32) -> Result<String, String>
    fn blocking_generate_at_time(thumb_dir: &Path, ffmpeg: &Path, path: &str, w: u32, h: u32, time: f64) -> Result<String, String>
}
```

**Note on `play_video`:** The actual process spawn in `play_video` is already non-blocking (`spawn()` not `output()`), but it calls `get_data_dir()` and constructs `generator` synchronously first. The async version avoids this.

### Module 3: Scanner + Thumbnail Integration

**Problem:** Currently the scanner generates thumbnails inline within `blocking_scan` for each found file, serializing I/O and subprocess work.

**Solution:** After `blocking_scan` collects all file paths and inserts them into the database:

1. The scanner returns both the count of files and a list of file paths that need thumbnail generation.
2. The caller (`start_scan` in `main_window.rs`) spawns a background task that processes thumbnails:
   ```rust
   let db = app.db.clone();
   let extensions = app.db.blocking_lock().get_video_extensions();
   let paths_to_scan = paths_to_generate_thumbnails; // from scan result

   std::thread::spawn(move || {
       let rt = tokio::runtime::Builder::new_current_thread()
           .enable_all().build().unwrap();
       rt.block_on(async {
           let generator = ThumbnailGenerator::new(&root_dir, &ffmpeg_path);
           for path in paths_to_scan {
               if let Ok(thumb) = generator.generate_thumbnail(&path).await {
                   // Update DB with thumbnail path
                   if let Some(file_id) = db.blocking_lock().find_file_by_path(&path).ok().flatten() {
                       db.blocking_lock().update_thumbnail(file_id, &thumb).ok();
                       // Send progress to UI via channel
                   }
               }
           }
       });
   });
   ```
   Wait — this still uses nested runtime. Better approach:
3. The background task uses `tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap()` or simply uses the spawning context's runtime if available. Since `start_scan` is called from the main thread (egui's `update` loop), we need the spawned std thread. The key improvement is: **scanner thread no longer creates a nested tokio runtime** — it directly runs blocking scan and thumbnail generation on OS threads via `spawn_blocking` from a multi-thread tokio runtime.

**Revised calling pattern:**
```rust
fn start_scan(app: &mut App) {
    // ... existing channel setup ...

    std::thread::spawn(move || {
        // Use existing tokio runtime from spawn context if called within tokio context,
        // OR create minimal runtime just for this work
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // The scan itself uses spawn_blocking internally
            let scanner = Scanner::new(db.clone(), extensions);
            let result = scanner.scan_path(path_id, &path).await;
            // ... report result via channel ...
        });
    });
}
```

The key architectural win: `scanner.scan_path` itself calls `spawn_blocking` internally, so the scanner no longer needs a tokio runtime. The std thread just needs a runtime to await the `spawn_blocking` task.

### Module 4: Fix ffmpeg Path Lookup

**Problem:** `get_ffmpeg_path()` is duplicated in 3 places:
- `ui/main_window.rs`
- `ui/context_menu.rs`
- `ui/thumbnail_grid.rs`

**Solution:** Move to a shared utility in `src/utils.rs` or `src/ffmpeg.rs`:
```rust
pub fn get_ffmpeg_path() -> Option<PathBuf> {
    let root_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("video-thumb-viewer");
    let ffmpeg_dir = root_dir.join("ffmpeg");
    #[cfg(target_os = "windows")] {
        let path = ffmpeg_dir.join("ffmpeg.exe");
        if path.exists() { return Some(path); }
        std::process::Command::new("where").arg("ffmpeg")
            .output().ok().filter(|o| o.status.success())
            .map(|_| PathBuf::from("ffmpeg"))
    }
    #[cfg(not(target_os = "windows"))] {
        let path = ffmpeg_dir.join("ffmpeg");
        if path.exists() { return Some(path); }
        Some(PathBuf::from("ffmpeg"))
    }
}
```

### Module 5: Unified Data Directory Function

**Problem:** `get_data_dir()` exists in multiple places, creating the same path multiple times.

**Solution:** Centralize in `src/utils.rs`:
```rust
use std::path::PathBuf;

pub fn local_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("video-thumb-viewer")
}
```

All callsites (`scanner/mod.rs`, `thumbnail/mod.rs`, `ui/main_window.rs`, `ui/context_menu.rs`, `ui/thumbnail_grid.rs`) replace their local implementations with `use crate::utils::local_data_dir`.

---

## Files to Create or Modify

| File | Action | Responsibility |
|------|--------|----------------|
| `src/scanner/mod.rs` | **Modify** | Refactor `scan_path` to use `spawn_blocking`; add `blocking_scan` helper |
| `src/thumbnail/mod.rs` | **Modify** | Add async methods + `blocking_*` factory functions; rename existing methods to `*_sync` |
| `src/utils.rs` | **Create** | Export `local_data_dir()` and `get_ffmpeg_path()` |
| `src/main.rs` | **Modify** | Add `mod utils;` |
| `src/app.rs` | **Modify** | Ensure `load_files_for_path` refreshes UI after scan completes |
| `src/ui/main_window.rs` | **Modify** | Use `utils::get_ffmpeg_path()`; remove duplicate `get_data_dir`; improve scan progress feedback |
| `src/ui/context_menu.rs` | **Modify** | Use `utils::get_ffmpeg_path()` and `utils::local_data_dir()`; async-ify screenshot capture |
| `src/ui/thumbnail_grid.rs` | **Modify** | Use `utils::get_ffmpeg_path()` and `utils::local_data_dir()` |

Note: `ui/thumbnail_grid.rs` is currently not wired into `main_window.rs`. It coexists with the inline thumbnail rendering in `show_thumbnail_item`. During this refactor, the inline code in `main_window.rs` should be migrated to call `ThumbnailGrid` properly, reducing code duplication.

---

## Error Handling

- `spawn_blocking` returns `JoinError` if the task panics. Wrap in `map_err` with context.
- `tokio::runtime::Runtime::new()` can fail at startup. Propagate the error (panic is acceptable at app startup for a debug build, log it).
- ffmpeg subprocess failures are already handled gracefully in the existing code (return `Err(String)`). The async wrappers preserve this behavior.
- If the database lock is poisoned, propagate the error. Use `expect()` only for truly catastrophic initialization failures.

---

## Testing Approaches

1. **Integration test:** Add a test that creates a temp directory with 3 video files, runs `Scanner::new(...).scan_path(...)`, verifies all 3 files are in the database. Use `tempfile::TempDir`.
2. **Thread safety:** Ensure no `&mut self` borrows span `await` points in any async method.
3. **Manual test scenario:**
   - Add a folder with 50+ video files
   - Click "更新資料庫"
   - Verify UI remains scrollable and responsive
   - Verify thumbnails appear progressively
   - Verify scan status updates in real-time

---

## Out of Scope

- Changing the database schema
- Adding new UI features (panels, dialogs, etc.)
- Changing the thumbnail generation algorithm or ffmpeg parameters
- Batch file operations (rename, move, delete)
- Export/import functionality