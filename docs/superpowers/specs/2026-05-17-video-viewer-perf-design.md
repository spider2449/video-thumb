# Video Viewer Performance at Scale — Two-Phase Design

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the app responsive and efficient with 1000+ videos, without a risky big-bang rewrite.

**Strategy:** Phase 1 eliminates the worst bottlenecks with minimal architectural change. Phase 2 introduces background workers to eliminate remaining UI blocking.

**Tech Stack:** Rust, egui/eframe, rusqlite, tokio, ffmpeg subprocess, walkdir.

---

## Problem Statement

At 1000+ videos, the application exhibits:
1. **Slow startup** — loads all files from DB on launch
2. **UI freezing** — `blocking_lock()` calls on the UI thread during scans and browsing
3. **Long scan times** — thumbnail generation runs inline during file scanning, serializing ffmpeg calls
4. **Duplicated code** — `get_ffmpeg_path()`, `get_data_dir()`, `hex_to_color()` exist in 3-5 files each
5. **No texture cache eviction** — GPU memory grows unbounded as more thumbnails are loaded
6. **Dead code** — `tab_bar.rs`, `thumbnail_grid.rs`, `add_path_dialog.rs` are unused

---

## Phase 1: Incremental Optimization

### 1.1 Lazy-Load Files Per Tab

**Current state:**
- `App::new()` calls `load_files_for_path()` for the first watched path
- `get_current_files()` reads from DB on every call, then filters/sorts in-memory
- This causes per-frame DB reads and slow startup

**New behavior:**
- Files are only loaded from DB when a tab is selected
- Results are cached in `HashMap<i64, Vec<FileEntry>>`
- Cache is invalidated when: filter/search/sort changes, or a scan completes
- `get_current_files()` reads from cache, applies filters in-memory, returns sorted results

**Files modified:**
- `src/app.rs` — change `load_files_for_path()` to lazy-load, add cache invalidation logic
- `src/ui/main_window.rs` — update tab switching to trigger lazy load

### 1.2 Separate Thumbnail Generation from Scanning

**Current state:**
- `Scanner::scan_path()` generates a thumbnail for every file during the scan loop
- Each ffmpeg call blocks the scan thread sequentially
- 1000 files × ~2s per ffmpeg call = ~33 minutes

**New behavior:**
- **Scan phase:** WalkDir → collect file metadata → upsert to DB only. No ffmpeg calls.
- **Thumbnail phase:** After scan completes, spawn a background thread that processes a queue of files needing thumbnails, updating DB as it goes.
- **Progress reporting:** Send periodic updates via channel so UI can show "Generating thumbnails: 45/1000".
- **Skip existing:** If a thumbnail file already exists on disk, skip generation for that file.

**Files modified:**
- `src/scanner/mod.rs` — remove thumbnail generation from scan loop, return list of files needing thumbnails
- `src/ui/main_window.rs` — spawn thumbnail worker thread after scan, handle progress updates
- `src/app.rs` — add `thumbnail_queue` and `thumbnail_progress` fields to `App` struct

### 1.3 Consolidate Duplicated Utilities

**Current state:**
- `get_ffmpeg_path()` exists in: `scanner/mod.rs`, `main_window.rs`, `thumbnail_grid.rs`, `context_menu.rs`
- `get_data_dir()` exists in: `main_window.rs`, `thumbnail_grid.rs`, `context_menu.rs`
- `hex_to_color()` exists in: `main_window.rs`, `context_menu.rs`, `tag_manager.rs`

**New behavior:**
- Create `src/utils.rs` module exporting:
  - `pub fn get_ffmpeg_path() -> Option<PathBuf>`
  - `pub fn get_data_dir() -> PathBuf`
  - `pub fn hex_to_color(hex: &str) -> egui::Color32`
- All call sites import from `crate::utils`
- Remove local implementations from all files

**Files created:**
- `src/utils.rs`

**Files modified:**
- `src/main.rs` — add `mod utils;`
- `src/scanner/mod.rs` — replace local `get_ffmpeg()` with `crate::utils::get_ffmpeg_path()`
- `src/ui/main_window.rs` — replace local implementations
- `src/ui/context_menu.rs` — replace local implementations
- `src/ui/thumbnail_grid.rs` — replace local implementations
- `src/ui/tag_manager.rs` — replace local `hex_to_color()`
- `src/ui/screenshot_dialog.rs` — replace local `get_ffmpeg()` and `get_data_dir()`

### 1.4 Enable SQLite WAL Mode + Remove Dead Code

**Changes:**
- Add `PRAGMA journal_mode = WAL;` to `Database::init_schema()` in `src/db/mod.rs`
- Delete unused files: `src/ui/tab_bar.rs`, `src/ui/thumbnail_grid.rs`, `src/ui/add_path_dialog.rs`
- Remove references from `src/ui/mod.rs`

**Files modified:**
- `src/db/mod.rs` — add WAL pragma
- `src/ui/mod.rs` — remove dead module declarations

**Files deleted:**
- `src/ui/tab_bar.rs`
- `src/ui/thumbnail_grid.rs`
- `src/ui/add_path_dialog.rs`

---

## Phase 2: Background Workers

### 2.1 DB Command Bus (DbActor)

**Current state:**
- Every UI action calls `app.db.blocking_lock()` directly
- DB operations block the UI thread for their duration
- `Arc<Mutex<Database>>` pattern is used throughout

**New behavior:**
- A `DbActor` struct runs on a dedicated background thread
- All DB operations are sent as commands via `mpsc` channel
- Results are returned via `oneshot` channels
- The UI sends commands and checks for responses each frame (non-blocking)
- If a response isn't ready, the UI shows a loading state

**Command types:**
```rust
enum DbCommand {
    GetFiles { path_id: i64, reply: oneshot::Sender<Result<Vec<FileEntry>>> },
    UpsertFile { file: FileEntry, reply: oneshot::Sender<Result<i64>> },
    UpdateRating { file_id: i64, rating: i32, reply: oneshot::Sender<Result<()>> },
    AddTagToFile { file_id: i64, tag_id: i64, reply: oneshot::Sender<Result<()>> },
    RemoveTagFromFile { file_id: i64, tag_id: i64, reply: oneshot::Sender<Result<()>> },
    GetTagsForFile { file_id: i64, reply: oneshot::Sender<Result<Vec<Tag>>> },
    GetAllTags { reply: oneshot::Sender<Result<Vec<Tag>>> },
    GetAllPaths { reply: oneshot::Sender<Result<Vec<WatchedPath>>> },
    AddPath { path: String, display_name: String, reply: oneshot::Sender<Result<i64>> },
    RemovePath { path_id: i64, reply: oneshot::Sender<Result<()>> },
    GetSetting { key: String, reply: oneshot::Sender<Result<Option<String>>> },
    SetSetting { key: String, value: String, reply: oneshot::Sender<Result<()>> },
    MoveFile { file_id: i64, new_path_id: i64, new_path: String, reply: oneshot::Sender<Result<()>> },
    DeleteFile { file_id: i64, reply: oneshot::Sender<Result<()>> },
    PathExists { path: String, reply: oneshot::Sender<Result<bool>> },
    Shutdown { reply: oneshot::Sender<()> },
}
```

**Files created:**
- `src/db/actor.rs` — `DbActor` struct and `DbCommand` enum

**Files modified:**
- `src/db/mod.rs` — add `mod actor;` and `DbActor` constructor
- `src/app.rs` — replace `Arc<Mutex<Database>>` with `DbHandle` (a thin wrapper around the `mpsc::Sender<DbCommand>`, providing convenience methods like `send_command(cmd)` that return `oneshot::Receiver`)
- `src/ui/main_window.rs` — update all DB calls to use command bus
- `src/ui/context_menu.rs` — update all DB calls to use command bus
- `src/ui/settings_dialog.rs` — update DB calls
- `src/ui/tag_manager.rs` — update DB calls
- `src/ui/screenshot_dialog.rs` — update DB calls
- `src/scanner/mod.rs` — update DB access pattern

### 2.2 Thumbnail Worker Pool

**Current state:**
- Thumbnails generated sequentially on a single background thread
- No parallelism, no rate limiting

**New behavior:**
- A pool of 2 worker threads processes thumbnail requests in parallel
- Each worker: takes file path from shared queue → runs ffmpeg → updates DB via DbActor → sends progress update
- Rate limited to max 2 concurrent ffmpeg processes to avoid CPU saturation
- Progress updates sent to UI via channel for real-time status display

**Files created:**
- `src/thumbnail/worker.rs` — `ThumbnailWorkerPool` struct

**Files modified:**
- `src/ui/main_window.rs` — use worker pool instead of single thread
- `src/app.rs` — add worker pool to App struct

### 2.3 Proper Texture Cache

**Current state:**
- `thread_local!` HashMap of `egui::TextureHandle`
- No eviction policy — GPU memory grows unbounded
- Potential cross-thread access issues

**New behavior:**
- Use `egui_extras::image` built-in image loader with `FileImageLoader`
- Automatic caching with LRU eviction
- Memory-bounded cache size (configurable, default 100MB)
- Thumbnails load on-demand as they scroll into view

**Dependencies added:**
- `egui_extras` with `file` feature

**Files modified:**
- `src/ui/main_window.rs` — replace `load_tex()` with `egui_extras::image`
- `Cargo.toml` — add `egui_extras` dependency

### 2.4 Virtual Scrolling (Optional)

**Current state:**
- All thumbnails rendered every frame, including off-screen ones
- Performance degrades linearly with file count

**New behavior:**
- Only render thumbnails visible in the viewport + small buffer (2 rows above/below)
- Uses `egui::ScrollArea` with manual item positioning based on scroll offset
- Grid layout calculated from available width and thumbnail size

**Decision point:** Implement only if Phase 1-2.3 doesn't provide sufficient performance at 5000+ files.

---

## Files Summary

| File | Phase | Action | Responsibility |
|------|-------|--------|----------------|
| `src/utils.rs` | 1.3 | Create | Shared utilities: ffmpeg path, data dir, hex_to_color |
| `src/db/actor.rs` | 2.1 | Create | DbActor command bus |
| `src/thumbnail/worker.rs` | 2.2 | Create | Thumbnail worker pool |
| `src/main.rs` | 1.3 | Modify | Add `mod utils;` |
| `src/app.rs` | 1.1, 1.2, 2.1, 2.2 | Modify | Lazy loading, cache, DbHandle, worker pool |
| `src/db/mod.rs` | 1.4, 2.1 | Modify | WAL mode, add actor module |
| `src/scanner/mod.rs` | 1.2, 1.3, 2.1 | Modify | Remove inline thumbnails, use utils, use DbActor |
| `src/ui/main_window.rs` | 1.1, 1.2, 1.3, 2.1, 2.2, 2.3 | Modify | All UI updates for new architecture |
| `src/ui/context_menu.rs` | 1.3, 2.1 | Modify | Use utils, use DbActor |
| `src/ui/tag_manager.rs` | 1.3, 2.1 | Modify | Use utils, use DbActor |
| `src/ui/screenshot_dialog.rs` | 1.3, 2.1 | Modify | Use utils, use DbActor |
| `src/ui/settings_dialog.rs` | 2.1 | Modify | Use DbActor |
| `src/ui/mod.rs` | 1.4 | Modify | Remove dead module declarations |
| `src/ui/tab_bar.rs` | 1.4 | Delete | Unused |
| `src/ui/thumbnail_grid.rs` | 1.4 | Delete | Unused |
| `src/ui/add_path_dialog.rs` | 1.4 | Delete | Unused |
| `Cargo.toml` | 2.3 | Modify | Add `egui_extras` dependency |

---

## Error Handling

- `spawn_blocking` returns `JoinError` if the task panics — wrap in `map_err` with context
- ffmpeg subprocess failures return `Err(String)` — preserve existing behavior
- Database lock poisoning — propagate error, log, do not panic
- Channel send failures — log warning, do not crash
- Thumbnail generation failures — skip file, log warning, continue with next

## Testing Approaches

1. **Integration test:** Create temp directory with test video files, run scanner, verify DB entries
2. **Manual test:** Add folder with 50+ video files, click "更新資料庫", verify UI remains responsive
3. **Manual test:** Verify thumbnails appear progressively during thumbnail phase
4. **Manual test:** Verify scan status updates in real-time
5. **Manual test:** Verify tab switching loads files lazily

## Out of Scope

- Database schema changes or migrations
- New UI features or visual changes
- Changing thumbnail generation algorithm or ffmpeg parameters
- Batch file operations (rename, move, delete)
- Export/import functionality
- Network sync or multi-user features
