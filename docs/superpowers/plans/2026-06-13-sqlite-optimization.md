# SQLite Performance Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Optimize SQLite database performance through PRAGMA tuning, batch transaction wrapping for bulk operations, and migration skip optimization.

**Architecture:** Keep `rusqlite` with single-connection `Arc<Mutex<Database>>` pattern. Add performance PRAGMAs at connection open. Wrap scanner bulk writes in a transaction. Skip completed thumbnail migration at startup. Add a `PRAGMA optimize` call on close.

**Tech Stack:** Rust, rusqlite 0.32 (bundled), existing tokio runtime

---

### Task 1: Add Performance PRAGMAs

**Files:**
- Modify: `src/db/mod.rs:15-28` (the `Database::new` method)

These PRAGMAs are set once at connection open and persist for the connection lifetime.

- [ ] **Step 1: Add PRAGMAs to `Database::new`**

Replace the current PRAGMA batch in `src/db/mod.rs` line 20:

```rust
conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
```

With:

```rust
conn.execute_batch(
    "PRAGMA foreign_keys = ON;
     PRAGMA journal_mode = WAL;
     PRAGMA synchronous = NORMAL;
     PRAGMA busy_timeout = 5000;
     PRAGMA cache_size = -64000;
     PRAGMA temp_store = MEMORY;
     PRAGMA mmap_size = 268435456;"
)?;
```

Explanation of each:
- `synchronous = NORMAL` — safe with WAL mode, much faster writes than FULL (default)
- `busy_timeout = 5000` — wait up to 5s for locks instead of failing immediately
- `cache_size = -64000` — 64MB page cache (negative = bytes)
- `temp_store = MEMORY` — temp tables/indices kept in RAM
- `mmap_size = 268435456` — allow memory-mapped I/O up to 256MB for reads

- [ ] **Step 2: Add same PRAGMAs to `new_in_memory` (test helper)**

In `src/db/mod.rs` line 33, replace:

```rust
conn.execute_batch("PRAGMA foreign_keys = ON;")?;
```

With:

```rust
conn.execute_batch(
    "PRAGMA foreign_keys = ON;
     PRAGMA synchronous = NORMAL;
     PRAGMA busy_timeout = 5000;
     PRAGMA cache_size = -64000;
     PRAGMA temp_store = MEMORY;"
)?;
```

(Note: `journal_mode` and `mmap_size` are not relevant for in-memory DBs.)

- [ ] **Step 3: Run tests to verify**

Run: `cargo test`
Expected: All existing tests pass. No behavior change — only performance.

- [ ] **Step 4: Commit**

```bash
git add src/db/mod.rs
git commit -m "perf: add SQLite PRAGMA tuning for WAL, cache, mmap, busy_timeout"
```

---

### Task 2: Wrap Scanner Bulk Writes in a Transaction

**Files:**
- Modify: `src/scanner/mod.rs:43-79` (the `scan_path` method's DB write section)

The scanner currently calls `db.upsert_file()` and `db.delete_file()` one-at-a-time without a transaction. Each individual write acquires/releases a WAL lock. Wrapping all writes in a single transaction reduces lock overhead from O(N) to O(1) and batches the WAL commits.

- [ ] **Step 1: Add `begin_transaction` and `commit_transaction` methods to Database**

Add to `src/db/mod.rs` (inside the `impl Database` block, after `new_in_memory`):

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

- [ ] **Step 2: Wrap scanner upserts and deletes in a transaction**

In `src/scanner/mod.rs`, replace lines 44-79 (the DB write block inside `scan_path`):

```rust
let db_files;
{
    let db = self.db.lock().unwrap();
    for file_entry in &files {
        if let Err(e) = db.upsert_file(&file_entry) {
            error!("Failed to insert file: {:?}", e);
        } else {
            count += 1;
        }
    }

    // Delete records for files that no longer exist on disk
    let found_paths: HashSet<String> = files.iter()
        .map(|f| f.file_path.clone())
        .collect();
    if let Ok(existing_files) = db.get_files_for_path(path_id) {
        let mut deleted = 0;
        for existing in &existing_files {
            if !found_paths.contains(&existing.file_path) {
                if let Err(e) = db.delete_file(existing.id) {
                    error!("Failed to delete stale record for {}: {:?}", existing.file_path, e);
                } else {
                    deleted += 1;
                }
            }
        }
        if deleted > 0 {
            info!("Deleted {} stale records for files no longer on disk", deleted);
        }
    }

    db_files = db.get_files_for_path(path_id).unwrap_or_default();
}
```

With:

```rust
let db_files;
{
    let db = self.db.lock().unwrap();
    if let Err(e) = db.begin_transaction() {
        error!("Failed to begin transaction: {:?}", e);
    }

    for file_entry in &files {
        if let Err(e) = db.upsert_file(&file_entry) {
            error!("Failed to insert file: {:?}", e);
        } else {
            count += 1;
        }
    }

    let found_paths: HashSet<String> = files.iter()
        .map(|f| f.file_path.clone())
        .collect();
    if let Ok(existing_files) = db.get_files_for_path(path_id) {
        let mut deleted = 0;
        for existing in &existing_files {
            if !found_paths.contains(&existing.file_path) {
                if let Err(e) = db.delete_file(existing.id) {
                    error!("Failed to delete stale record for {}: {:?}", existing.file_path, e);
                } else {
                    deleted += 1;
                }
            }
        }
        if deleted > 0 {
            info!("Deleted {} stale records for files no longer on disk", deleted);
        }
    }

    if let Err(e) = db.commit_transaction() {
        error!("Failed to commit transaction: {:?}", e);
    }

    db_files = db.get_files_for_path(path_id).unwrap_or_default();
}
```

- [ ] **Step 3: Run tests to verify**

Run: `cargo test`
Expected: All tests pass. Scanner transaction wrapping doesn't affect correctness of individual operations.

- [ ] **Step 4: Commit**

```bash
git add src/db/mod.rs src/scanner/mod.rs
git commit -m "perf: wrap scanner bulk writes in a single transaction"
```

---

### Task 3: Skip Completed Thumbnail Migration

**Files:**
- Modify: `src/db/mod.rs:116-166` (the `migrate_thumbnails` method)

Currently `migrate_thumbnails` runs `SELECT COUNT(*) FROM files WHERE thumbnail_path IS NOT NULL` on every startup. Once migration is complete (no rows with `thumbnail_path IS NOT NULL` remain), this query returns 0 and the method returns early — but it still executes the query. We can persist a flag in the `settings` table to skip this entirely after the first successful migration.

- [ ] **Step 1: Add migration skip logic to `migrate_thumbnails`**

Replace `src/db/mod.rs` lines 116-166:

```rust
fn migrate_thumbnails(&self) -> SqliteResult<()> {
    let count: i64 = self.conn.query_row(
        "SELECT COUNT(*) FROM files WHERE thumbnail_path IS NOT NULL",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        return Ok(());
    }

    let mut stmt = self.conn.prepare(
        "SELECT id, thumbnail_path FROM files WHERE thumbnail_path IS NOT NULL AND id NOT IN (SELECT file_id FROM thumbnail_map)"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
        ))
    })?;

    let thumbs_dir = crate::utils::get_data_dir().join("thumbs");
    std::fs::create_dir_all(&thumbs_dir).ok();

    for row in rows {
        let (file_id, thumb_path) = row?;
        let old_path = std::path::PathBuf::from(&thumb_path);

        if old_path.exists() {
            let uuid = uuid::Uuid::new_v4().to_string();
            let new_filename = format!("{}.jpg", uuid);
            let new_path = thumbs_dir.join(&new_filename);

            if std::fs::rename(&old_path, &new_path).is_ok() {
                self.conn.execute(
                    "INSERT OR REPLACE INTO thumbnail_map (file_id, thumb_filename) VALUES (?1, ?2)",
                    params![file_id, new_filename],
                )?;
                self.conn.execute(
                    "UPDATE files SET thumbnail_path = NULL WHERE id = ?1",
                    [file_id],
                )?;
                info!("Migrated thumbnail for file_id={}: {} -> {}", file_id, thumb_path, new_filename);
            } else {
                log::warn!("Failed to rename thumbnail for file_id={}", file_id);
            }
        }
    }

    Ok(())
}
```

With:

```rust
fn migrate_thumbnails(&self) -> SqliteResult<()> {
    if self.get_setting("thumbnail_migration_done")?.is_some() {
        return Ok(());
    }

    let count: i64 = self.conn.query_row(
        "SELECT COUNT(*) FROM files WHERE thumbnail_path IS NOT NULL",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        self.set_setting("thumbnail_migration_done", "1")?;
        return Ok(());
    }

    let mut stmt = self.conn.prepare(
        "SELECT id, thumbnail_path FROM files WHERE thumbnail_path IS NOT NULL AND id NOT IN (SELECT file_id FROM thumbnail_map)"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
        ))
    })?;

    let thumbs_dir = crate::utils::get_data_dir().join("thumbs");
    std::fs::create_dir_all(&thumbs_dir).ok();

    for row in rows {
        let (file_id, thumb_path) = row?;
        let old_path = std::path::PathBuf::from(&thumb_path);

        if old_path.exists() {
            let uuid = uuid::Uuid::new_v4().to_string();
            let new_filename = format!("{}.jpg", uuid);
            let new_path = thumbs_dir.join(&new_filename);

            if std::fs::rename(&old_path, &new_path).is_ok() {
                self.conn.execute(
                    "INSERT OR REPLACE INTO thumbnail_map (file_id, thumb_filename) VALUES (?1, ?2)",
                    params![file_id, new_filename],
                )?;
                self.conn.execute(
                    "UPDATE files SET thumbnail_path = NULL WHERE id = ?1",
                    [file_id],
                )?;
                info!("Migrated thumbnail for file_id={}: {} -> {}", file_id, thumb_path, new_filename);
            } else {
                log::warn!("Failed to rename thumbnail for file_id={}", file_id);
            }
        }
    }

    self.set_setting("thumbnail_migration_done", "1")?;
    Ok(())
}
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test`
Expected: All tests pass. Since in-memory test DBs start with no data, `count == 0` branch sets the flag and returns early.

- [ ] **Step 3: Commit**

```bash
git add src/db/mod.rs
git commit -m "perf: skip thumbnail migration when already completed"
```

---

### Task 4: Add `PRAGMA optimize` on Database Close

**Files:**
- Modify: `src/db/mod.rs:200-204` (the `Drop` implementation)

SQLite's `PRAGMA optimize` updates internal statistics so the query planner makes better choices. Running it on close is the recommended pattern from SQLite documentation — it's lightweight and only does work when statistics are stale.

- [ ] **Step 1: Add `PRAGMA optimize` to `Database::drop`**

Replace `src/db/mod.rs` lines 200-204:

```rust
impl Drop for Database {
    fn drop(&mut self) {
        info!("Closing database connection");
    }
}
```

With:

```rust
impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.conn.execute_batch("PRAGMA optimize");
        info!("Closing database connection");
    }
}
```

(The `let _ =` ignores the result — we don't want to panic in a Drop impl, and a failed PRAGMA optimize is non-critical.)

- [ ] **Step 2: Run tests to verify**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/db/mod.rs
git commit -m "perf: run PRAGMA optimize on database close"
```

---

### Task 5: Add `file_name` Index for Search Performance

**Files:**
- Modify: `src/db/mod.rs:88-91` (the `init_schema` method — index creation section)

The app supports searching files by name (`self.search_query`). With large libraries, a case-insensitive search via `LIKE` on `file_path` benefits from an index. SQLite doesn't support `LIKE` index optimization for case-insensitive patterns directly, but an index on `file_name` helps with exact/sorted lookups and future filter features.

- [ ] **Step 1: Add `file_name` index to `init_schema`**

In `src/db/mod.rs`, after line 91 (`CREATE INDEX IF NOT EXISTS idx_file_tags_tag ON file_tags(tag_id);`), add:

```sql
CREATE INDEX IF NOT EXISTS idx_files_file_name ON files(file_name);
CREATE INDEX IF NOT EXISTS idx_files_file_ext ON files(file_ext);
```

So the full `init_schema` batch becomes:

```rust
fn init_schema(&self) -> SqliteResult<()> {
    info!("Creating database schema");
    self.conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS watched_paths (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT UNIQUE NOT NULL,
            display_name TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path_id INTEGER NOT NULL REFERENCES watched_paths(id) ON DELETE CASCADE,
            file_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_ext TEXT,
            thumbnail_path TEXT,
            file_size INTEGER,
            modified_at DATETIME,
            rating INTEGER DEFAULT 0 CHECK(rating >= 0 AND rating <= 5),
            has_custom_thumb BOOLEAN DEFAULT FALSE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(path_id, file_path)
        );

        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL,
            color TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS file_tags (
            file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (file_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS thumbnail_map (
            file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
            thumb_filename TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_files_path_id ON files(path_id);
        CREATE INDEX IF NOT EXISTS idx_files_rating ON files(rating);
        CREATE INDEX IF NOT EXISTS idx_file_tags_file ON file_tags(file_id);
        CREATE INDEX IF NOT EXISTS idx_file_tags_tag ON file_tags(tag_id);
        CREATE INDEX IF NOT EXISTS idx_files_file_name ON files(file_name);
        CREATE INDEX IF NOT EXISTS idx_files_file_ext ON files(file_ext);
        ",
    )?;
    Ok(())
}
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test`
Expected: All tests pass. New indexes are created with `IF NOT EXISTS` so they're harmless for existing DBs.

- [ ] **Step 3: Commit**

```bash
git add src/db/mod.rs
git commit -m "perf: add indexes on files.file_name and files.file_ext"
```

---

## Summary of Expected Performance Gains

| Optimization | Impact | Where Felt |
|-------------|--------|------------|
| `synchronous = NORMAL` | **High** — WAL writes ~2-3x faster | Scanner bulk inserts, all writes |
| Scanner transaction wrapping | **High** — eliminates per-statement WAL lock overhead | Scanning large directories |
| `busy_timeout = 5000` | **Medium** — prevents sporadic lock failures | Concurrent scanner + UI access |
| `cache_size = -64000` | **Medium** — fewer disk reads for hot data | `get_files_for_path`, tag queries |
| `mmap_size = 256MB` | **Medium** — faster reads via OS page cache | All read queries on large DBs |
| `temp_store = MEMORY` | **Low-Medium** — temp tables in RAM | Complex queries with sorts/joins |
| Migration skip | **Low** — avoids one COUNT(*) query per startup | App startup |
| `PRAGMA optimize` on close | **Low** — better query plans over time | Long-term query performance |
| `file_name`/`file_ext` indexes | **Low-Medium** — helps sorted/filter queries | File search and filtering |
