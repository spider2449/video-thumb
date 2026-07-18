# Scan/Thumbnail Lifecycle Correctness Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix seven correctness issues in the scan → thumbnail-generation → cleanup lifecycle: button-gating races, non-transactional scan writes, a stale-id bug in `upsert_file`, orphaned thumbnail files, a stale status message, and four clippy warnings.

**Architecture:** Targeted fixes following the codebase's existing patterns (channel polling, `safe_lock`, i18n via `lang::t`). The scanner's manual `BEGIN`/`COMMIT` is replaced by one new transactional method `Database::sync_files_for_path`. UI buttons are gated on a new `App::is_background_busy()` helper. No structural refactor.

**Tech Stack:** Rust, egui/eframe 0.31, rusqlite 0.32 (bundled SQLite, `RETURNING` supported).

**Spec:** `docs/superpowers/specs/2026-07-18-scan-thumbnail-correctness-design.md`

## Global Constraints

- ALL code, comments, and identifiers in English only (user's global CLAUDE.md).
- Per AGENTS.md: version in `Cargo.toml` must be bumped `0.1.6` → `0.1.7` (done in Task 8).
- Verification commands: `cargo check`, `cargo test`, `cargo clippy`. After Task 7, clippy must report zero warnings.
- All 38 existing tests must stay green throughout.
- Thumbnail filenames read from the DB must pass the existing path-traversal check (reject names containing `/`, `\`, or `..`) before being used in filesystem operations.
- Do not use interactive git commands. Commit after each task.

**Pre-existing working-tree state:** `AGENTS.md` has an uncommitted doc fix (data locations). Task 1 Step 1 commits it first so later commits stay clean.

---

### Task 1: `upsert_file` returns correct id on update path

**Files:**
- Modify: `src/db/queries.rs:73-96` (function `upsert_file`)
- Test: `src/db/queries.rs` (tests module, after `test_upsert_file_updates_existing`)

**Interfaces:**
- Consumes: existing `Database`, `FileEntry`, test helpers `test_db()`, `test_file()` in the same file.
- Produces: `pub fn upsert_file(&self, file: &FileEntry) -> SqliteResult<i64>` — same signature, but the returned id is now correct on the `ON CONFLICT ... DO UPDATE` path. Task 2's tests rely on this correctness.

- [ ] **Step 1: Commit the pre-existing AGENTS.md fix**

```bash
git add AGENTS.md
git commit -m "docs: correct data locations in AGENTS.md

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

- [ ] **Step 2: Write the failing test**

In `src/db/queries.rs`, inside `mod tests`, after `test_upsert_file_updates_existing` (around line 550), add:

```rust
    #[test]
    fn test_upsert_file_returns_same_id_on_update() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();

        let mut file = test_file(path_id, "/videos/a.mp4");
        let id1 = db.upsert_file(&file).unwrap();

        // Insert a different file so last_insert_rowid() moves past id1.
        db.upsert_file(&test_file(path_id, "/videos/b.mp4")).unwrap();

        file.file_size = 999;
        let id2 = db.upsert_file(&file).unwrap();
        assert_eq!(id1, id2);
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test test_upsert_file_returns_same_id_on_update`
Expected: FAIL — `assertion `left == right` failed` (id2 is b.mp4's rowid with the old `last_insert_rowid()` implementation).

- [ ] **Step 4: Rewrite `upsert_file` to use `RETURNING id`**

Replace the whole function (`src/db/queries.rs:73-96`) with:

```rust
    #[allow(dead_code)]
    pub fn upsert_file(&self, file: &FileEntry) -> SqliteResult<i64> {
        self.conn.query_row(
            "INSERT INTO files (path_id, file_path, file_name, file_ext, thumbnail_path, file_size, modified_at, rating, has_custom_thumb)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(path_id, file_path) DO UPDATE SET
                file_name = excluded.file_name,
                thumbnail_path = COALESCE(excluded.thumbnail_path, files.thumbnail_path),
                file_size = excluded.file_size,
                modified_at = excluded.modified_at
             RETURNING id",
            params![
                file.path_id,
                file.file_path,
                file.file_name,
                file.file_ext,
                file.thumbnail_path,
                file.file_size,
                file.modified_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                file.rating,
                file.has_custom_thumb,
            ],
            |row| row.get(0),
        )
    }
```

- [ ] **Step 5: Run tests to verify all pass**

Run: `cargo test`
Expected: 39 passed; 0 failed.

- [ ] **Step 6: Commit**

```bash
git add src/db/queries.rs
git commit -m "fix: upsert_file returns correct id on conflict-update path

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: Transactional `Database::sync_files_for_path`

**Files:**
- Modify: `src/db/queries.rs` (refactor `upsert_files_batch` at lines 98-148, add `SyncStats` + `sync_files_for_path` after it)
- Test: `src/db/queries.rs` (tests module)

**Interfaces:**
- Consumes: `FileEntry`, existing chunked-upsert SQL, `rusqlite::Connection::transaction()`.
- Produces (Task 3 depends on these exact signatures):

```rust
pub struct SyncStats {
    pub upserted: usize,
    pub stale_deleted: usize,
    /// Thumb filenames of deleted stale records; caller removes these files from disk.
    pub orphan_thumb_files: Vec<String>,
}

impl Database {
    pub fn sync_files_for_path(
        &mut self,
        path_id: i64,
        files: &[FileEntry],
        delete_stale: bool,
    ) -> SqliteResult<SyncStats>
}
```

`SyncStats` must be re-exported from the `db` module: it lives in `queries.rs` which is `mod queries;` (private) in `src/db/mod.rs`, so add a `pub use` (Step 3).

- [ ] **Step 1: Write the failing tests**

In `src/db/queries.rs` tests module, after `test_batch_upsert_updates_existing`, add:

```rust
    #[test]
    fn test_sync_files_upserts_and_deletes_stale() {
        let mut db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();

        let old = test_file(path_id, "/videos/old.mp4");
        let old_id = db.upsert_file(&old).unwrap();
        db.set_thumb_filename(old_id, "old-thumb.jpg").unwrap();

        let new_files = vec![test_file(path_id, "/videos/new.mp4")];
        let stats = db.sync_files_for_path(path_id, &new_files, true).unwrap();

        assert_eq!(stats.upserted, 1);
        assert_eq!(stats.stale_deleted, 1);
        assert_eq!(stats.orphan_thumb_files, vec!["old-thumb.jpg".to_string()]);

        let files = db.get_files_for_path(path_id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_path, "/videos/new.mp4");
    }

    #[test]
    fn test_sync_files_keeps_stale_when_delete_disabled() {
        let mut db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        db.upsert_file(&test_file(path_id, "/videos/old.mp4")).unwrap();

        let new_files = vec![test_file(path_id, "/videos/new.mp4")];
        let stats = db.sync_files_for_path(path_id, &new_files, false).unwrap();

        assert_eq!(stats.stale_deleted, 0);
        assert!(stats.orphan_thumb_files.is_empty());
        assert_eq!(db.get_files_for_path(path_id).unwrap().len(), 2);
    }

    #[test]
    fn test_sync_files_ignores_other_paths() {
        let mut db = test_db();
        let p1 = db.add_path("/a", "A").unwrap();
        let p2 = db.add_path("/b", "B").unwrap();
        db.upsert_file(&test_file(p2, "/b/keep.mp4")).unwrap();

        let stats = db
            .sync_files_for_path(p1, &[test_file(p1, "/a/one.mp4")], true)
            .unwrap();

        assert_eq!(stats.stale_deleted, 0);
        assert_eq!(db.get_files_for_path(p2).unwrap().len(), 1);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_sync_files`
Expected: COMPILE ERROR — `no method named sync_files_for_path found`.

- [ ] **Step 3: Implement**

In `src/db/queries.rs`:

3a. Extract the chunked-upsert body into a free function so both the old batch method and the new transaction can use it. Replace `upsert_files_batch` (lines 98-148) with:

```rust
    pub fn upsert_files_batch(&self, files: &[FileEntry]) -> SqliteResult<()> {
        upsert_files_chunked(&self.conn, files)
    }
```

and add at file scope (outside `impl Database`, e.g. directly above `// ============ Files ============`):

```rust
const UPSERT_BATCH_SIZE: usize = 500;

fn upsert_files_chunked(conn: &rusqlite::Connection, files: &[FileEntry]) -> SqliteResult<()> {
    if files.is_empty() {
        return Ok(());
    }

    for chunk in files.chunks(UPSERT_BATCH_SIZE) {
        let mut sql = String::from(
            "INSERT INTO files (path_id, file_path, file_name, file_ext, thumbnail_path, file_size, modified_at, rating, has_custom_thumb) VALUES "
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(chunk.len() * 9);

        for (i, file) in chunk.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            let offset = i * 9;
            sql.push_str(&format!(
                "(?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{})",
                offset + 1,
                offset + 2,
                offset + 3,
                offset + 4,
                offset + 5,
                offset + 6,
                offset + 7,
                offset + 8,
                offset + 9
            ));

            params.push(Box::new(file.path_id));
            params.push(Box::new(file.file_path.clone()));
            params.push(Box::new(file.file_name.clone()));
            params.push(Box::new(file.file_ext.clone()));
            params.push(Box::new(file.thumbnail_path.clone()));
            params.push(Box::new(file.file_size));
            params.push(Box::new(file.modified_at.format("%Y-%m-%d %H:%M:%S").to_string()));
            params.push(Box::new(file.rating));
            params.push(Box::new(file.has_custom_thumb));
        }

        sql.push_str(" ON CONFLICT(path_id, file_path) DO UPDATE SET file_name = excluded.file_name, thumbnail_path = COALESCE(excluded.thumbnail_path, files.thumbnail_path), file_size = excluded.file_size, modified_at = excluded.modified_at");

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_ref.as_slice())?;
    }

    Ok(())
}
```

3b. Add `SyncStats` and `sync_files_for_path` inside the same `impl Database` block, directly after `upsert_files_batch` (struct goes at file scope above the impl):

```rust
/// Result of syncing scanned files into the database.
pub struct SyncStats {
    pub upserted: usize,
    pub stale_deleted: usize,
    /// Thumb filenames of deleted stale records; caller removes these files from disk.
    pub orphan_thumb_files: Vec<String>,
}
```

```rust
    /// Upsert scanned files and (optionally) delete records for files no longer
    /// on disk, all inside one transaction. Pass `delete_stale = false` when the
    /// scan was cancelled part-way, so missing files are not treated as deleted.
    pub fn sync_files_for_path(
        &mut self,
        path_id: i64,
        files: &[FileEntry],
        delete_stale: bool,
    ) -> SqliteResult<SyncStats> {
        let tx = self.conn.transaction()?;

        upsert_files_chunked(&tx, files)?;

        let mut stale_deleted = 0;
        let mut orphan_thumb_files = Vec::new();

        if delete_stale {
            let found: std::collections::HashSet<&str> =
                files.iter().map(|f| f.file_path.as_str()).collect();

            let existing: Vec<(i64, String, Option<String>)> = {
                let mut stmt = tx.prepare(
                    "SELECT f.id, f.file_path, tm.thumb_filename
                     FROM files f
                     LEFT JOIN thumbnail_map tm ON tm.file_id = f.id
                     WHERE f.path_id = ?1",
                )?;
                let rows = stmt
                    .query_map([path_id], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })?
                    .collect::<SqliteResult<Vec<_>>>()?;
                rows
            };

            for (file_id, file_path, thumb_filename) in existing {
                if !found.contains(file_path.as_str()) {
                    tx.execute("DELETE FROM files WHERE id = ?1", [file_id])?;
                    stale_deleted += 1;
                    if let Some(filename) = thumb_filename {
                        orphan_thumb_files.push(filename);
                    }
                }
            }
        }

        tx.commit()?;

        Ok(SyncStats {
            upserted: files.len(),
            stale_deleted,
            orphan_thumb_files,
        })
    }
```

Note: `tx.prepare(...)` works because `Transaction` derefs to `Connection`. The inner block scoping around `existing` is required so `stmt` (which borrows `tx`) is dropped before `tx.execute` / `tx.commit`.

3c. In `src/db/mod.rs`, `queries` is a private module and `SyncStats` must be visible to the scanner. Change line 2 area:

```rust
mod models;
mod queries;
```

to:

```rust
mod models;
mod queries;

pub use queries::SyncStats;
```

(If the compiler instead reports `SyncStats` as unreachable/private via a different path, the equivalent fix is `pub mod queries;` — prefer the `pub use`.)

- [ ] **Step 4: Run tests to verify all pass**

Run: `cargo test`
Expected: 42 passed; 0 failed. (`cargo check` may warn that `SyncStats`/`sync_files_for_path` are unused outside tests — that resolves in Task 3; do not add `#[allow(dead_code)]`.)

- [ ] **Step 5: Commit**

```bash
git add src/db/queries.rs src/db/mod.rs
git commit -m "feat: add transactional sync_files_for_path with stale-record cleanup

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: Scanner uses `sync_files_for_path`; deletes orphan thumbs; propagates DB errors

**Files:**
- Modify: `src/scanner/mod.rs:45-108` (the DB-write section of `scan_path`)
- Modify: `src/db/mod.rs:63-71` (delete `begin_transaction` and `commit_transaction`)

**Interfaces:**
- Consumes: `Database::sync_files_for_path(&mut self, path_id: i64, files: &[FileEntry], delete_stale: bool) -> SqliteResult<SyncStats>` and `SyncStats { upserted, stale_deleted, orphan_thumb_files }` from Task 2.
- Produces: `Scanner::scan_path` keeps its signature `pub async fn scan_path(&self, path_id: i64, root_path: &str) -> Result<ScanFileResult, String>` but now returns `Err` when the DB sync fails (previously it logged and reported success). The UI already handles `Err` via `ScanResult { success: false }` in `main_window.rs:646-655` — no UI change needed.

- [ ] **Step 1: Replace the DB-write section of `scan_path`**

In `src/scanner/mod.rs`, replace lines 45-105 (from the comment `// Insert files into database first ...` through the `for file_entry in &files { ... }` loop that fills `files_needing_thumbnails`) with:

```rust
        // Sync scanned files into the database in one transaction.
        // Skip stale-record deletion when the scan was cancelled or found
        // nothing, to avoid treating unscanned files as deleted.
        let delete_stale = scan_completed && !files.is_empty();
        let (stats, db_files) = {
            let mut db = self.db.safe_lock();
            let stats = db
                .sync_files_for_path(path_id, &files, delete_stale)
                .map_err(|e| format!("Database sync failed: {e}"))?;
            let db_files = db.get_files_for_path(path_id).unwrap_or_default();
            (stats, db_files)
        };
        let count = stats.upserted;
        if stats.stale_deleted > 0 {
            info!("Deleted {} stale records for files no longer on disk", stats.stale_deleted);
        }
        if !delete_stale && !scan_completed {
            info!("Scan was cancelled; skipped stale-record cleanup to avoid data loss");
        }

        // Remove thumbnail files orphaned by the stale-record deletion (lock released).
        let data_dir = crate::utils::get_data_dir();
        let thumbs_dir = data_dir.join("thumbs");
        for filename in &stats.orphan_thumb_files {
            if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
                error!("Invalid thumbnail filename from DB, not deleting: {}", filename);
                continue;
            }
            let path = thumbs_dir.join(filename);
            if path.exists() {
                if let Err(e) = std::fs::remove_file(&path) {
                    error!("Failed to delete orphan thumbnail {}: {}", filename, e);
                }
            }
        }

        // Check thumbnail existence using file_ids from the DB.
        let mut files_needing_thumbnails = Vec::new();
        let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &PathBuf::from(""), self.db.clone());

        let path_to_id: std::collections::HashMap<String, i64> = db_files.iter()
            .map(|f| (f.file_path.clone(), f.id))
            .collect();
        for file_entry in &files {
            let file_id = path_to_id.get(&file_entry.file_path).copied();
            let thumb_path = gen.get_thumbnail_path(&file_entry.file_path, file_id);
            if !thumb_path.exists() {
                files_needing_thumbnails.push(file_entry.file_path.clone());
            }
        }
```

The final `info!` + `Ok(ScanFileResult { ... })` lines at the end of the function stay unchanged. The now-unused `use std::collections::HashSet;` import at the top of the file must be removed (the `HashSet` moved into `queries.rs`).

- [ ] **Step 2: Delete the manual transaction methods**

In `src/db/mod.rs`, delete lines 63-71 entirely:

```rust
    pub fn begin_transaction(&self) -> SqliteResult<()> {
        self.conn.execute_batch("BEGIN")?;
        Ok(())
    }

    pub fn commit_transaction(&self) -> SqliteResult<()> {
        self.conn.execute_batch("COMMIT")?;
        Ok(())
    }
```

- [ ] **Step 3: Verify build and tests**

Run: `cargo check`
Expected: success, no warnings about unused `sync_files_for_path` (now used), no other callers of the deleted methods (compiler would error).

Run: `cargo test`
Expected: 42 passed; 0 failed.

- [ ] **Step 4: Commit**

```bash
git add src/scanner/mod.rs src/db/mod.rs
git commit -m "fix: scanner writes via real transaction, deletes orphan thumbs, surfaces DB errors

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: Gate scan/cleanup buttons on background-busy state

**Files:**
- Modify: `src/app.rs` (add method to `impl App`, e.g. after `refresh_cache_if_needed` around line 265)
- Modify: `src/ui/main_window.rs:30-44` (toolbar buttons)

**Interfaces:**
- Consumes: existing `App` fields `scanning`, `thumbnail_progress`, `clean_thumbs_rx`.
- Produces: `pub fn is_background_busy(&self) -> bool` on `App`. Fires for scanning, thumbnail generation, and thumbnail cleanup; deliberately excludes `loading_files` (read-only background load must not block anything).

- [ ] **Step 1: Add the helper to `App`**

In `src/app.rs`, after `refresh_cache_if_needed` (around line 265), add:

```rust
    /// True while a scan, thumbnail generation, or thumbnail cleanup is running.
    /// Deliberately excludes `loading_files`: a background file load is
    /// read-only and must not block starting new work.
    pub fn is_background_busy(&self) -> bool {
        self.scanning
            || self.thumbnail_progress.is_some()
            || self.clean_thumbs_rx.is_some()
    }
```

- [ ] **Step 2: Gate both buttons**

In `src/ui/main_window.rs`, replace lines 30-44:

```rust
                    let scan_text = if app.scanning {
                        crate::lang::t(lang, "scanning").to_string()
                    } else {
                        crate::lang::t(lang, "update_db").to_string()
                    };

                    ui.add_enabled_ui(!app.scanning, |ui| {
                        if ui.button(&scan_text).clicked() {
                            Self::start_scan(app);
                        }
                    });

                    if ui.button(crate::lang::t(lang, "rescan_thumbs")).clicked() {
                        Self::clean_unused_thumbnails(app);
                    }
```

with:

```rust
                    let busy = app.is_background_busy();
                    let scan_text = if busy {
                        crate::lang::t(lang, "scanning").to_string()
                    } else {
                        crate::lang::t(lang, "update_db").to_string()
                    };

                    ui.add_enabled_ui(!busy, |ui| {
                        if ui.button(&scan_text).clicked() {
                            Self::start_scan(app);
                        }
                        if ui.button(crate::lang::t(lang, "rescan_thumbs")).clicked() {
                            Self::clean_unused_thumbnails(app);
                        }
                    });
```

- [ ] **Step 3: Verify build and tests**

Run: `cargo check` then `cargo test`
Expected: builds clean; 42 passed.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/ui/main_window.rs
git commit -m "fix: disable scan and cleanup buttons while background work runs

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: Cleanup only deletes `.jpg` files

**Files:**
- Modify: `src/ui/main_window.rs` (inside `clean_unused_thumbnails`, the `for entry in entries` loop, around lines 580-590)

**Interfaces:**
- Consumes: nothing new.
- Produces: no API change; behavior change only.

- [ ] **Step 1: Add the extension filter**

In `clean_unused_thumbnails`, replace:

```rust
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let filename = entry.file_name().to_string_lossy().to_string();
                if !referenced.contains(&filename)
                    && std::fs::remove_file(entry.path()).is_ok() {
                        deleted += 1;
                    }
            }
```

with:

```rust
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let filename = entry.file_name().to_string_lossy().to_string();
                // Only ever delete thumbnail images; leave any other file alone.
                if !filename.to_lowercase().ends_with(".jpg") {
                    continue;
                }
                if !referenced.contains(&filename)
                    && std::fs::remove_file(entry.path()).is_ok() {
                        deleted += 1;
                    }
            }
```

- [ ] **Step 2: Verify build and tests**

Run: `cargo check` then `cargo test`
Expected: builds clean; 42 passed.

- [ ] **Step 3: Commit**

```bash
git add src/ui/main_window.rs
git commit -m "fix: thumbnail cleanup only deletes .jpg files

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 6: Accurate thumbnail-completion status message

**Files:**
- Modify: `src/lang.rs` (add key inside the `TRANSLATIONS` Lazy block)
- Modify: `src/app.rs:520-527` (the `thumb_done_rx` `Ok(path_id)` arm in `update_impl`)

**Interfaces:**
- Consumes: `crate::lang::t(lang, key)`.
- Produces: new i18n key `thumbs_done`.

- [ ] **Step 1: Add the i18n key**

In `src/lang.rs`, after the `generating_thumbs_sep` line (line 128), add:

```rust
    m.insert("thumbs_done", [(Lang::ZhTw, "縮圖生成完成"), (Lang::En, "Thumbnail generation complete")].into_iter().collect());
```

(Chinese here is user-facing UI copy in the app's supported locale, matching every other entry in this table — not a source-code comment, so it does not violate the English-only code rule.)

- [ ] **Step 2: Use it in the completion handler**

In `src/app.rs`, replace the `Ok(path_id)` arm of the `thumb_done_rx` match (lines 522-527):

```rust
                Ok(path_id) => {
                    self.thumbnail_progress = None;
                    self.scan_status = format!("{}{}{}", crate::lang::t(self.lang, "done_found_prefix"), self.files.get(&path_id).map(|f| f.len()).unwrap_or(0), crate::lang::t(self.lang, "done_found_suffix"));
                    self.load_files_for_path(path_id);
                    ctx.request_repaint();
                }
```

with:

```rust
                Ok(path_id) => {
                    self.thumbnail_progress = None;
                    self.scan_status = crate::lang::t(self.lang, "thumbs_done").to_string();
                    self.load_files_for_path(path_id);
                    ctx.request_repaint();
                }
```

- [ ] **Step 3: Verify build and tests**

Run: `cargo check` then `cargo test`
Expected: builds clean; 42 passed.

- [ ] **Step 4: Commit**

```bash
git add src/lang.rs src/app.rs
git commit -m "fix: thumbnail completion message no longer shows stale file count

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 7: Clippy cleanup (sort_by_key + type aliases)

**Files:**
- Modify: `src/app.rs:14-15,36` (field types), `src/app.rs:239-243` (sorts)

**Interfaces:**
- Consumes: existing `App` fields.
- Produces: `pub type FilterCacheEntry` and `pub type FilesLoadMsg` in `src/app.rs` (same shapes as the current inline types — no call-site changes needed).

- [ ] **Step 1: Fix the two sorts**

In `src/app.rs:239-243`, replace:

```rust
        match self.sort_by {
            SortBy::Name => files.sort_by(|a, b| a.file_path.cmp(&b.file_path)),
            SortBy::Date => files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at)),
            SortBy::Rating => files.sort_by(|a, b| b.rating.cmp(&a.rating)),
        }
```

with:

```rust
        match self.sort_by {
            SortBy::Name => files.sort_by(|a, b| a.file_path.cmp(&b.file_path)),
            SortBy::Date => files.sort_by_key(|f| std::cmp::Reverse(f.modified_at)),
            SortBy::Rating => files.sort_by_key(|f| std::cmp::Reverse(f.rating)),
        }
```

(`SortBy::Name` stays `sort_by` — clippy only flags the reversed comparisons.)

- [ ] **Step 2: Add the type aliases**

In `src/app.rs`, above `pub struct App` (after the `use` block), add:

```rust
/// Cache entry for the current filtered+sorted view:
/// (path_id, sort, search query, rating filter, tag filter, result).
pub type FilterCacheEntry = (i64, SortBy, String, Option<i32>, Option<i64>, Arc<Vec<FileEntry>>);

/// Payload sent by the background file-load thread:
/// (path_id, files, file_id -> tag_ids map).
pub type FilesLoadMsg = (i64, Vec<FileEntry>, HashMap<i64, Vec<i64>>);
```

Then change the two fields:

```rust
    pub filter_cache: Option<(i64, SortBy, String, Option<i32>, Option<i64>, Arc<Vec<FileEntry>>)>,
```
→
```rust
    pub filter_cache: Option<FilterCacheEntry>,
```

and

```rust
    pub files_load_rx: Option<std::sync::mpsc::Receiver<(i64, Vec<crate::db::FileEntry>, std::collections::HashMap<i64, Vec<i64>>)>>,
```
→
```rust
    pub files_load_rx: Option<std::sync::mpsc::Receiver<FilesLoadMsg>>,
```

Note: `SortBy` is defined below `App` in the same file — Rust doesn't care about item order, so the alias can reference it.

- [ ] **Step 3: Verify zero clippy warnings**

Run: `cargo clippy`
Expected: `Finished` with **zero** warnings.

Run: `cargo test`
Expected: 42 passed.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "chore: fix clippy warnings (sort_by_key, type aliases)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 8: Version bump and final verification

**Files:**
- Modify: `Cargo.toml:3` (`version = "0.1.6"` → `version = "0.1.7"`)

**Interfaces:** none.

- [ ] **Step 1: Bump the version**

In `Cargo.toml`, change:

```toml
version = "0.1.6"
```

to:

```toml
version = "0.1.7"
```

- [ ] **Step 2: Full verification**

Run, in order:

```bash
cargo check
cargo test
cargo clippy
```

Expected: check clean, 42 tests pass, zero clippy warnings.

- [ ] **Step 3: Manual smoke test (human checkpoint)**

Build and run `cargo run`, then verify with a real watched folder:
1. Click "Update Database" — during the scan AND the following thumbnail generation, both "Update Database" and the cleanup button stay disabled.
2. When generation finishes, the status bar shows the "Thumbnail generation complete" message (or the ZhTw equivalent) and both buttons re-enable.
3. Delete a video file from the watched folder on disk, rescan — the record disappears and its thumbnail file is removed from the thumbs dir.
4. Drop a `notes.txt` into the thumbs dir, run cleanup — the file survives.

If any check fails, stop and debug before committing (use superpowers:systematic-debugging).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.1.7

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```
