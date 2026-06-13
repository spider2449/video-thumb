# AGENTS.md

## Git Rules

- **Version bumping**: Every fix must increment the version in `Cargo.toml` (e.g., `0.1.3` → `0.1.4`)

## Commands

```bash
cargo check          # Verify build
cargo test           # Run tests (mostly in src/db/queries.rs)
cargo clippy         # Lint
cargo build --release  # Build release binary
```

## Architecture

- **GUI**: egui/eframe (immediate mode), entry point `src/main.rs`
- **State**: `App` struct in `src/app.rs` holds all UI state, passed to UI modules
- **Database**: SQLite via rusqlite, schema in `src/db/mod.rs`, queries in `src/db/queries.rs`
- **Async work**: Background threads for scanning (`src/scanner/`) and thumbnail generation (`src/thumbnail/`)
- **UI modules**: `src/ui/main_window.rs` (grid layout), `context_menu.rs`, `settings_dialog.rs`, `tag_manager.rs`, `screenshot_dialog.rs`

## Key Patterns

- **i18n**: Use `crate::lang::t(app.lang, "key")` for all user-visible strings. Translations defined in `src/lang.rs`
- **Mutex access**: Use `db.safe_lock()` (via `SafeMutexExt` trait) instead of `db.lock().unwrap()`
- **Channel pattern**: Background threads send results via `mpsc::channel`. Poll with `try_recv()` in `update_impl()`. Handle `Disconnected` to avoid stuck state
- **Cache invalidation**: Set `self.cache_invalidated = true` after DB changes; `refresh_cache_if_needed()` reloads files
- **Virtual scrolling**: Grid renders only visible rows + buffer. See `show_main()` in `main_window.rs`

## Data Locations

- Database: `%LOCALAPPDATA%/video-thumb-viewer/data.db`
- Thumbnails: `%LOCALAPPDATA%/video-thumb-viewer/thumbs/`
- Config: `%LOCALAPPDATA%/video-thumb-viewer/config.json`

## Testing

- Tests use `Database::new_in_memory()` (defined in `src/db/mod.rs`)
- Most tests are in `src/db/queries.rs` covering CRUD operations
- Run specific test: `cargo test <test_name>`

## Gotchas

- `loading_files` flag stays true during background DB load - don't use for cursor state
- `scanning` flag controls Wait cursor - set false on completion/failure/thread death
- Thumbnail generation runs in separate thread, progress polled via channels
- Search debouncing: `search_last_changed` tracks input time, cache invalidated after 200ms idle
