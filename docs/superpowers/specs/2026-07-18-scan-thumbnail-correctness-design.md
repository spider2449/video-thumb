# Scan/Thumbnail Lifecycle Correctness Fixes — Design

Date: 2026-07-18
Status: Approved

## Background

A full codebase review found seven correctness-related issues clustered around the
scan → thumbnail-generation → cleanup lifecycle:

- **(a)** The "Update Database" button is re-enabled as soon as `scanning` goes false,
  while thumbnail generation continues in a background thread. Starting a second scan
  spawns a second generator; the old one keeps running with dropped channels.
- **(b)** "Clean thumbnails" can run concurrently with generation and delete a thumbnail
  in the window between ffmpeg writing it and `set_thumb_filename` committing. It also
  deletes *any* unrecognized file in the thumbs dir, not just thumbnails.
- **(c)** `Database::upsert_file` returns `last_insert_rowid()`, which is stale when the
  `ON CONFLICT ... DO UPDATE` path fires — the caller gets the wrong id on update.
- **(d)** The scanner wraps its writes in raw `BEGIN`/`COMMIT` batches with no rollback:
  a failed batch insert is logged and then committed anyway; a failed `COMMIT` leaves the
  shared connection stuck inside an open transaction.
- **(e)** When the scanner deletes stale records (files no longer on disk), the
  `thumbnail_map` row cascades but the thumbnail file on disk is orphaned until the user
  manually runs cleanup.
- **(f)** The thumbnail-completion status message reports the file count from the
  pre-reload cache, which is stale.
- **(g)** Four clippy warnings: two `unnecessary_sort_by`, two `type_complexity`.

## Decisions

- Scope: fix all of a–g in one pass.
- UX for (a): while any background work runs, the "Update Database" and
  "Clean thumbnails" buttons are **disabled** (no cancel, no queueing). The status bar
  already shows progress, so the user can see why.
- Approach: targeted fixes following existing patterns; no state-machine refactor of the
  channel-polling fields.

## Design

### 1. Busy-state gating (fixes a, most of b)

Add to `App` (`src/app.rs`):

```rust
pub fn is_background_busy(&self) -> bool {
    self.scanning
        || self.thumbnail_progress.is_some()
        || self.clean_thumbs_rx.is_some()
}
```

In `src/ui/main_window.rs`, gate both the "Update Database" button (currently gated only
on `scanning`) and the "Clean thumbnails" button (currently ungated) with
`ui.add_enabled_ui(!app.is_background_busy(), ...)`.

`loading_files` is deliberately excluded: a background file load is read-only and must
not block anything.

### 2. Transactional scan sync (fixes d, e)

New method in `src/db/queries.rs`:

```rust
pub struct SyncStats {
    pub upserted: usize,
    pub stale_deleted: usize,
    /// thumb filenames of deleted stale records; caller removes these from disk
    pub orphan_thumb_files: Vec<String>,
}

pub fn sync_files_for_path(
    &mut self,
    path_id: i64,
    files: &[FileEntry],
    delete_stale: bool,
) -> SqliteResult<SyncStats>
```

Behavior:

1. Opens a real `rusqlite::Transaction` (drop = rollback).
2. Runs the chunked batch upsert (moved from `upsert_files_batch`, or refactored so the
   chunk logic executes against the transaction).
3. If `delete_stale`: finds DB records for `path_id` whose `file_path` is not in the
   scanned set, collects their `thumbnail_map.thumb_filename` values, deletes the
   records.
4. Commits and returns `SyncStats`.

Scanner (`src/scanner/mod.rs`) replaces its entire `begin_transaction` /
`upsert_files_batch` / stale-delete / `commit_transaction` block with one call. After
the DB lock is released, it deletes each `orphan_thumb_files` entry from the thumbs
directory, applying the existing path-traversal filename check
(reject names containing `/`, `\`, or `..`).

`Database::begin_transaction` and `commit_transaction` are removed (the scanner was the
only caller). `sync_files_for_path` takes `&mut self` because
`Connection::transaction()` requires it; the scanner already holds the mutex guard, so
this is only a signature change.

The `delete_stale` flag carries the existing cancellation semantics: stale-record
cleanup is skipped when the scan was cancelled part-way (`scan_completed == false`) or
when the scan found zero files.

### 3. `upsert_file` id fix (fixes c)

Rewrite `upsert_file` to append `RETURNING id` and fetch it with `query_row`. The
bundled SQLite in rusqlite 0.32 is well past 3.35, so `RETURNING` is available.

Regression test: upsert the same `(path_id, file_path)` twice; assert both calls return
the same id.

### 4. Cleanup hardening (rest of b)

In `clean_unused_thumbnails` (`src/ui/main_window.rs`), skip any directory entry whose
file name does not end in `.jpg` (case-insensitive) before considering deletion. The
mid-generation race is closed by section 1's button gating, since cleanup and generation
can no longer be started concurrently.

### 5. Status message fix (fixes f)

On `thumb_done_rx` completion in `update_impl`, replace the
"Done! Found {stale count} files" message with a new i18n key:

- `thumbs_done`: ZhTw "縮圖生成完成", En "Thumbnail generation complete"

This removes the dependency on the not-yet-reloaded file count entirely. The subsequent
`load_files_for_path` still refreshes the grid and the status-bar file count.

### 6. Clippy cleanup (fixes g)

- `src/app.rs:241-242`: convert the two `sort_by` calls to
  `sort_by_key(|f| std::cmp::Reverse(...))`.
- Add type aliases for the two flagged complex types (the `filter_cache` tuple and the
  `files_load_rx` payload), e.g.:

```rust
pub type FilterKey = (i64, SortBy, String, Option<i32>, Option<i64>);
pub type FilesLoadMsg = (i64, Vec<FileEntry>, HashMap<i64, Vec<i64>>);
```

## Error handling

- If `sync_files_for_path` fails, the scanner returns an error through the existing
  `ScanResult { success: false, message }` path so the failure is visible in the status
  bar. (Previously DB errors were only logged and the scan reported success.)
- The transaction guarantees no partial commit: either all upserts and stale deletions
  land, or none do.
- Disk deletion of orphan thumbnails is best-effort: failures are logged, never fatal.

## Testing

- All 38 existing tests must stay green.
- New tests in `src/db/queries.rs`:
  - `upsert_file` returns the same id on insert and on conflict-update (section 3).
  - `sync_files_for_path`: upserts new files, deletes stale records, returns their thumb
    filenames in `orphan_thumb_files`, and leaves other paths' records untouched.
  - `sync_files_for_path` with `delete_stale = false` keeps stale records.
- Button gating and status messages are UI behavior with no test harness; verify
  manually: start a scan on a large folder, confirm both buttons stay disabled through
  thumbnail generation, and confirm the completion message.
- Per AGENTS.md: bump version to 0.1.7; `cargo check`, `cargo test`, `cargo clippy`
  (zero warnings expected after section 6).

## Out of scope

- Async thumbnail decoding on the UI thread (separate perf project).
- Removing the tokio dependency (separate simplification project).
- The clone-heavy in-memory updates in `context_menu.rs` (perf, not correctness).
