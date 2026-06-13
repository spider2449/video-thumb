# Codebase Improvement Design Spec

**Date**: 2026-06-13  
**Status**: Approved for Implementation  
**Scope**: 21 issues across critical, medium, and minor severity

---

## Overview

This spec defines a three-phase improvement plan for the Video Thumb Viewer codebase. Each phase produces a working build with all 38 existing tests passing. Phases are ordered by severity: critical fixes first, then medium improvements, then minor cleanup.

---

## Phase 1: Critical/Important Fixes

### 1. Config Caching

**Problem**: `get_data_dir()` re-reads `config.json` from disk on every call. Called in thumbnail path resolution, scanner, and other hot paths.

**Solution**: 
- Add `OnceLock<Config>` in `utils.rs` to cache config at startup
- `get_data_dir()` reads from cached config
- Settings changes (in Settings dialog) trigger config reload + cache update

**Files Modified**:
- `src/utils.rs`: Add `static CONFIG_CACHE: OnceLock<Config>`, initialize in `main.rs`
- `src/main.rs`: Call `utils::init_config()` after creating config
- `src/ui/settings_dialog.rs`: Call `utils::reload_config()` after saving

**Testing**: Verify config changes persist, data dir updates correctly

---

### 2. Mutex Poisoning Handling

**Problem**: All `.lock().unwrap()` calls will panic if mutex is poisoned (e.g., after a thread panic). This crashes the app.

**Solution**: 
- Create helper method `Database::safe_lock()` that returns `MutexGuard` even if poisoned
- Replace all `db.lock().unwrap()` with `db.safe_lock()`
- Implementation: `.lock().unwrap_or_else(|e| e.into_inner())`

**Files Modified**:
- `src/app.rs`: Update all `self.db.lock().unwrap()` calls
- `src/ui/*.rs`: Update all `app.db.lock().unwrap()` calls
- `src/scanner/mod.rs`: Update DB lock calls
- `src/thumbnail/mod.rs`: Update DB lock calls

**Testing**: Verify app doesn't crash on simulated panic scenarios

---

### 3. UI Thread Blocking

**Problem**: `load_files_for_path` and `clean_unused_thumbnails` run DB queries and filesystem I/O on the main UI thread, causing freezes during tab switches and thumbnail cleanup.

**Solution**: 
- `load_files_for_path`: Run DB query in `std::thread::spawn`, send results via `mpsc::channel`, show spinner while loading
- `clean_unused_thumbnails`: Move to background thread, update status via channel
- Poll channels in `update_impl` to handle results

**Files Modified**:
- `src/app.rs`: Refactor `load_files_for_path` to async pattern with channels
- `src/ui/main_window.rs`: Refactor `clean_unused_thumbnails` to background thread
- Add loading state flags and result receivers to `App` struct

**Testing**: Verify tab switches are responsive, no UI freezes during large folder loads

---

### 4. Texture Cache LRU Eviction

**Problem**: `texture_cache` is unbounded `HashMap`. Large folders consume all GPU memory with no eviction.

**Solution**: 
- Add `lru` crate to `Cargo.toml`
- Replace `HashMap<String, TextureHandle>` with `LruCache<String, TextureHandle>`
- Set capacity to 200 entries (configurable via settings if needed)
- When cache is full, oldest unused texture is evicted and GPU memory freed

**Files Modified**:
- `Cargo.toml`: Add `lru = "0.12"`
- `src/app.rs`: Change `texture_cache` type, update `load_tex` and `clear_texture_cache`
- `src/ui/main_window.rs`: Update `load_tex` signature

**Testing**: Verify texture cache doesn't grow unbounded, old textures are evicted

---

### 5. Deduplicate `play_video`

**Problem**: Nearly identical `play_video` implementations in `app.rs:271-303` and `thumbnail/mod.rs:262-296`.

**Solution**: 
- Remove `ThumbnailGenerator::play_video` method
- Keep single implementation in `App::play_video`
- `context_menu.rs` already calls through `app.play_video_file`, no changes needed

**Files Modified**:
- `src/app.rs`: Keep `play_video` method
- `src/thumbnail/mod.rs`: Remove `play_video` method

**Testing**: Verify video playback still works from context menu

---

### 6. Window State Persistence

**Problem**: `set_window_state` exists but is never called. Window position/size lost on exit.

**Solution**: 
- In `eframe::App::on_exit`, capture current window rect via `ctx.screen_rect()`
- Save via `config.set_window_state()` + `config.save()`
- Remove unused `set_window_state` warning

**Files Modified**:
- `src/app.rs`: Implement `on_exit` to save window state
- `src/config.rs`: Already has `set_window_state`, no changes needed

**Testing**: Verify window position/size persists across app restarts

---

### 7. Scan Cancellation

**Problem**: If user switches tabs or starts new scan, previous scan runs to completion with no way to abort.

**Solution**: 
- Add `Arc<AtomicBool>` cancellation flag to `Scanner`
- Check flag in `blocking_scan` loop (every 100 files)
- When user starts new scan, set flag to cancel previous
- Store cancellation flag in `App` struct

**Files Modified**:
- `src/scanner/mod.rs`: Add `cancel_flag: Arc<AtomicBool>` to `Scanner`, check in loop
- `src/app.rs`: Store `scan_cancel_flag: Arc<AtomicBool>`, set to true before starting new scan
- `src/ui/main_window.rs`: Set cancel flag in `start_scan`

**Testing**: Verify scan can be cancelled, no orphaned background threads

---

### 8. Transaction Safety

**Problem**: `remove_path` uses `unchecked_transaction()` which skips recursion depth check.

**Solution**: 
- Replace `unchecked_transaction()` with `transaction()` in `remove_path`
- No reason to skip recursion depth check

**Files Modified**:
- `src/db/queries.rs`: Change `unchecked_transaction()` to `transaction()`

**Testing**: Verify `remove_path` still works, all cascade deletes succeed

---

## Phase 2: Medium Issues

### 9. Arc Clone Optimization

**Problem**: `get_current_files` clones entire Vec on every filter/sort change. For large collections this is expensive.

**Solution**: 
- Cache filtered/sorted results separately
- Use `Arc::make_mut` for copy-on-write semantics when possible
- Only clone when filters actually change
- Add `filter_cache_key` to track when cache is valid

**Files Modified**:
- `src/app.rs`: Optimize `get_current_files` to avoid unnecessary clones

**Testing**: Verify filtering/sorting still works correctly, performance improved

---

### 10. Date Type Safety

**Problem**: Dates stored as `String` in models. No type safety, parsing/formatting at DB boundaries.

**Solution**: 
- Replace `String` dates with `chrono::DateTime<Utc>` in `FileEntry` and `WatchedPath`
- Update DB queries to parse/format at boundaries
- Existing DB stores as TEXT, parse on read, format on write

**Files Modified**:
- `src/db/models.rs`: Change `created_at`, `modified_at` types
- `src/db/queries.rs`: Update queries to parse/format dates
- `src/scanner/mod.rs`: Update `create_file_entry` to use `DateTime<Utc>`
- `src/app.rs`: Update date comparisons

**Testing**: Verify dates display correctly, sorting by date works

---

### 11. i18n Refactor

**Problem**: 300-line match statement in `lang.rs`, hard to maintain.

**Solution**: 
- Use `once_cell::sync::Lazy` with `HashMap<Lang, HashMap<&str, &str>>`
- Initialize once at startup
- Simpler than external files, still compile-time checked

**Files Modified**:
- `Cargo.toml`: Add `once_cell = "1.19"`
- `src/lang.rs`: Refactor to use HashMap lookup

**Testing**: Verify all UI strings display correctly in both languages

---

### 12. Rc<RefCell> Cleanup

**Problem**: `show_thumbnail_item` uses `Rc<RefCell<Option<Response>>>` workaround for borrow checker.

**Solution**: 
- Restructure to avoid needing thumb_rect outside the closure
- Allocate response separately, or pass thumb_rect as parameter
- Cleaner code, no runtime borrow checking overhead

**Files Modified**:
- `src/ui/main_window.rs`: Refactor `show_thumbnail_item` to avoid Rc<RefCell>

**Testing**: Verify thumbnail clicks, double-clicks, right-clicks all work

---

### 13. Screenshot Temp File Cleanup

**Problem**: Writes `preview_{file_id}.jpg` to data dir, orphaned on crash.

**Solution**: 
- Use `tempfile` crate for automatic cleanup
- Or: write to system temp dir

**Recommendation**: `tempfile` crate (idiomatic, cross-platform)

**Files Modified**:
- `Cargo.toml`: Add `tempfile = "3.10"`
- `src/ui/screenshot_dialog.rs`: Use `tempfile::NamedTempFile` for previews

**Testing**: Verify no orphaned preview files after app restart

---

### 14. Color Preset Click Detection

**Problem**: Manual hover/click detection with `ui.input()` in `tag_manager.rs`.

**Solution**: 
- Use `ui.add(Button::new("").fill(color))` for each preset
- More idiomatic egui, better accessibility, less code

**Files Modified**:
- `src/ui/tag_manager.rs`: Refactor `show_color_presets` to use Button widgets

**Testing**: Verify color presets are clickable, selected color updates correctly

---

### 15. Search Debouncing

**Problem**: Every keystroke triggers `invalidate_cache()`, causing re-filter on each character.

**Solution**: 
- Add debounce timer (200ms)
- Only invalidate if search query hasn't changed for debounce duration
- Use `Instant::now()` to track last change time

**Files Modified**:
- `src/app.rs`: Add `search_last_changed: Instant` field
- `src/ui/main_window.rs`: Check debounce before invalidating cache

**Testing**: Verify search still works, reduced CPU usage during typing

---

## Phase 3: Minor Cleanup

### 16. Dead Code Cleanup

**Problem**: 13 dead-code warnings from unused methods, fields, structs.

**Solution**: 
- Remove unused methods: `Tag::new`, `FileWithTags`, `thumbnail_exists`, `ThumbnailGenerator::play_video`, `get_file_duration`, `upsert_file`, `get_file_by_id`, `tag_name_exists`, `add_tag_internal`, `capture_screenshot_internal`, `parse_time_string`
- Remove unused fields: `scan_complete_tx`, `ContextMenu::add_tag_name`, `ContextMenu::time_input`
- Mark `created_at` fields with `#[allow(dead_code)]` if kept for future use

**Files Modified**:
- `src/db/models.rs`: Remove `Tag::new`, `FileWithTags`; add `#[allow(dead_code)]` to `created_at` fields
- `src/db/queries.rs`: Remove `upsert_file`, `get_file_by_id`, `tag_name_exists`
- `src/scanner/mod.rs`: Remove `get_file_duration`
- `src/thumbnail/mod.rs`: Remove `thumbnail_exists`, `play_video`
- `src/ui/context_menu.rs`: Remove unused fields and methods

**Testing**: Verify `cargo check` has no warnings

---

### 17. Indentation Fix

**Problem**: Inconsistent indentation at `app.rs:118`.

**Solution**: 
- Fix indentation in `App::new` initialization

**Files Modified**:
- `src/app.rs`: Fix line 118 indentation

**Testing**: Visual verification

---

### 18. `winresource` Conditional Compilation

**Problem**: `winresource` in `build-dependencies` is Windows-only but declared unconditionally.

**Solution**: 
- Move `winresource` to `[target.'cfg(windows)'.build-dependencies]`
- Update `build.rs` to only run on Windows

**Files Modified**:
- `Cargo.toml`: Move `winresource` to Windows-specific section
- `build.rs`: Add `#[cfg(windows)]` guard

**Testing**: Verify build works on Windows, no errors on other platforms

---

### 19. `md5` Crate Update

**Problem**: `md5 = "0.7"` last updated 2021.

**Solution**: 
- Keep `md5` crate (stable, no breaking changes needed)
- Alternative: `md-5` from RustCrypto (actively maintained) but migration not worth the effort

**Recommendation**: No action needed

**Files Modified**: None

**Testing**: N/A

---

### 20. CI/CD Configuration

**Problem**: No CI/CD configuration for automated testing.

**Solution**: 
- Add GitHub Actions workflow for:
  - `cargo check` on push/PR
  - `cargo test` on push/PR
  - `cargo clippy` for lint warnings
  - Build release binaries for Windows/Linux/macOS

**Files Modified**:
- `.github/workflows/ci.yml`: New file

**Testing**: Verify workflow runs on GitHub

---

### 21. Error Handling Improvements (Optional)

**Problem**: `.ok()` calls suppress errors, no structured error types.

**Solution**: 
- Replace `.ok()` calls with proper error logging
- Add `thiserror` crate for structured error types
- Improve error messages for user-facing operations

**Files Modified**:
- `Cargo.toml`: Add `thiserror = "1.0"`
- Multiple files: Replace `.ok()` with `.map_err()` + logging

**Testing**: Verify errors are logged, user sees helpful messages

---

## Success Criteria

- All 38 existing tests pass
- `cargo check` produces no warnings
- UI is responsive during large folder loads
- Texture cache doesn't grow unbounded
- Window position/size persists across restarts
- Scans can be cancelled
- No orphaned temp files after crashes
- Code is cleaner and more maintainable

---

## Implementation Order

1. **Phase 1** (Critical): Issues 1-8
2. **Phase 2** (Medium): Issues 9-15
3. **Phase 3** (Minor): Issues 16-21

Each phase is a separate commit. Phase 1 should be completed and verified before starting Phase 2.

---

## Dependencies

- `lru = "0.12"` (Phase 1, Issue 4)
- `once_cell = "1.19"` (Phase 2, Issue 11)
- `tempfile = "3.10"` (Phase 2, Issue 13)
- `thiserror = "1.0"` (Phase 3, Issue 21 - optional)

---

## Risk Assessment

- **Low Risk**: Most changes are refactoring with clear before/after behavior
- **Medium Risk**: UI thread blocking changes require careful channel management
- **Testing**: All changes should maintain backward compatibility with existing DB schema

---

## Notes

- Phase 1 Issue 2 (mutex poisoning) may require careful testing to ensure no regressions
- Phase 2 Issue 10 (date types) requires DB migration logic or careful parsing
- Phase 3 Issue 21 (error handling) is optional and can be deferred if time-constrained
