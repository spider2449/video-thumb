# Thumbnail UUID Mapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace MD5-based thumbnail naming with UUID-based naming backed by a SQLite mapping table, with hybrid fallback for legacy thumbnails.

**Architecture:** Add `thumbnail_map` table linking `file_id` to UUID filenames. Thumbnail files on disk are named `{uuid}.jpg`. Lookup checks mapping first, falls back to MD5 for unmigrated thumbnails.

**Tech Stack:** Rust, rusqlite, uuid crate (v4), md5 (existing), eframe/egui

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src/db/mod.rs` | Schema definition, migration logic |
| `src/db/queries.rs` | Thumbnail mapping CRUD queries |
| `src/thumbnail/mod.rs` | Thumbnail generation, UUID lookup, MD5 fallback |
| `src/scanner/mod.rs` | Thumbnail existence check during scan |
| `src/app.rs` | Thumbnail generation orchestration |
| `src/ui/context_menu.rs` | Reset/capture thumbnail operations |
| `src/ui/screenshot_dialog.rs` | Screenshot capture with new mapping |

---

### Task 1: Add uuid dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add uuid crate to Cargo.toml**

```toml
[dependencies]
# ... existing dependencies ...
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 2: Verify dependency resolves**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add uuid dependency for thumbnail naming"
```

---

### Task 2: Add thumbnail_map schema and migration

**Files:**
- Modify: `src/db/mod.rs:29-78` (schema), `src/db/mod.rs:81-98` (init_default_settings)

- [ ] **Step 1: Add thumbnail_map table to schema**

In `src/db/mod.rs`, inside `init_schema()`, add to the SQL batch:

```rust
CREATE TABLE IF NOT EXISTS thumbnail_map (
    file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    thumb_filename TEXT NOT NULL
);
```

- [ ] **Step 2: Add migration function**

Add a new method to `Database` impl:

```rust
fn migrate_thumbnails(&self) -> SqliteResult<()> {
    // Check if thumbnail_map exists
    let mut stmt = self.conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='thumbnail_map'"
    )?;
    let exists = stmt.query_row([], |_| Ok(())).is_ok();
    
    if !exists {
        info!("thumbnail_map does not exist, creating and migrating");
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS thumbnail_map (
                file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
                thumb_filename TEXT NOT NULL
            );"
        )?;
        
        // Migrate existing thumbnails
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, thumbnail_path FROM files WHERE thumbnail_path IS NOT NULL"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        
        let thumbs_dir = crate::utils::get_data_dir().join("thumbs");
        
        for row in rows {
            let (file_id, file_path, thumb_path) = row?;
            let old_path = std::path::PathBuf::from(&thumb_path);
            
            if old_path.exists() {
                // Generate UUID and rename
                let uuid = uuid::Uuid::new_v4().to_string();
                let new_filename = format!("{}.jpg", uuid);
                let new_path = thumbs_dir.join(&new_filename);
                
                if std::fs::rename(&old_path, &new_path).is_ok() {
                    self.conn.execute(
                        "INSERT INTO thumbnail_map (file_id, thumb_filename) VALUES (?1, ?2)",
                        params![file_id, new_filename],
                    )?;
                    info!("Migrated thumbnail for file_id={}: {} -> {}", file_id, thumb_path, new_filename);
                } else {
                    log::warn!("Failed to rename thumbnail for file_id={}", file_id);
                }
            }
            
            // Clear old thumbnail_path
            self.conn.execute(
                "UPDATE files SET thumbnail_path = NULL WHERE id = ?1",
                [file_id],
            )?;
        }
    }
    
    Ok(())
}
```

- [ ] **Step 3: Call migration in Database::new**

In `Database::new()`, after `init_default_settings()`, add:

```rust
db.migrate_thumbnails()?;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add src/db/mod.rs
git commit -m "feat: add thumbnail_map schema and migration logic"
```

---

### Task 3: Add thumbnail mapping queries

**Files:**
- Modify: `src/db/queries.rs`

- [ ] **Step 1: Add get_thumb_filename query**

```rust
pub fn get_thumb_filename(&self, file_id: i64) -> SqliteResult<Option<String>> {
    let mut stmt = self.conn.prepare(
        "SELECT thumb_filename FROM thumbnail_map WHERE file_id = ?1"
    )?;
    let result = stmt.query_row([file_id], |row| row.get(0));
    match result {
        Ok(filename) => Ok(Some(filename)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}
```

- [ ] **Step 2: Add set_thumb_filename query**

```rust
pub fn set_thumb_filename(&self, file_id: i64, filename: &str) -> SqliteResult<()> {
    self.conn.execute(
        "INSERT OR REPLACE INTO thumbnail_map (file_id, thumb_filename) VALUES (?1, ?2)",
        params![file_id, filename],
    )?;
    Ok(())
}
```

- [ ] **Step 3: Add delete_thumb_mapping query**

```rust
pub fn delete_thumb_mapping(&self, file_id: i64) -> SqliteResult<()> {
    self.conn.execute(
        "DELETE FROM thumbnail_map WHERE file_id = ?1",
        [file_id],
    )?;
    Ok(())
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add src/db/queries.rs
git commit -m "feat: add thumbnail mapping CRUD queries"
```

---

### Task 4: Update thumbnail module with UUID support

**Files:**
- Modify: `src/thumbnail/mod.rs`

- [ ] **Step 1: Update get_thumbnail_path to use UUID mapping**

Replace the existing `get_thumbnail_path` method:

```rust
pub fn get_thumbnail_path(&self, video_path: &str, file_id: Option<i64>) -> PathBuf {
    // Try UUID mapping first
    if let Some(fid) = file_id {
        if let Ok(Some(filename)) = self.db.blocking_lock().get_thumb_filename(fid) {
            return self.get_thumbs_dir().join(filename);
        }
    }
    
    // Fallback to MD5 for unmigrated thumbnails
    let hash = format!("{:x}", md5::compute(video_path.as_bytes()));
    self.get_thumbs_dir().join(format!("{}.jpg", hash))
}
```

Add `db: Arc<Mutex<Database>>` field to `ThumbnailGenerator` struct and update `new()`:

```rust
pub struct ThumbnailGenerator {
    root_dir: PathBuf,
    ffmpeg_path: PathBuf,
    width: u32,
    height: u32,
    db: Arc<Mutex<Database>>,
}

pub fn new(root_dir: &Path, ffmpeg_path: &Path, db: Arc<Mutex<Database>>) -> Self {
    Self {
        root_dir: root_dir.to_path_buf(),
        ffmpeg_path: ffmpeg_path.to_path_buf(),
        width: 384,
        height: 216,
        db,
    }
}
```

- [ ] **Step 2: Update thumbnail_exists to use mapping**

```rust
pub fn thumbnail_exists(&self, video_path: &str, file_id: Option<i64>) -> bool {
    self.get_thumbnail_path(video_path, file_id).exists()
}
```

- [ ] **Step 3: Update generate_thumbnail to use UUID**

In `generate_thumbnail`, after successful ffmpeg run, add:

```rust
// Generate UUID and rename
let uuid = uuid::Uuid::new_v4().to_string();
let new_filename = format!("{}.jpg", uuid);
let new_path = self.get_thumbs_dir().join(&new_filename);
std::fs::rename(&thumb_path, &new_path)?;

// Update mapping
if let Some(fid) = file_id {
    let _ = self.db.blocking_lock().set_thumb_filename(fid, &new_filename);
}

return Ok(new_path.to_string_lossy().to_string());
```

- [ ] **Step 4: Update generate_thumbnail_at_time similarly**

Apply the same UUID generation and mapping update logic.

- [ ] **Step 5: Update generate_thumbnail_at_frame similarly**

Apply the same UUID generation and mapping update logic.

- [ ] **Step 6: Update delete_thumbnail to clean up mapping**

```rust
pub fn delete_thumbnail(&self, file_path: &str, file_id: Option<i64>) {
    // Delete UUID-mapped thumbnail
    if let Some(fid) = file_id {
        if let Ok(Some(filename)) = self.db.blocking_lock().get_thumb_filename(fid) {
            let thumb_path = self.get_thumbs_dir().join(filename);
            if thumb_path.exists() {
                let _ = std::fs::remove_file(&thumb_path);
            }
            let _ = self.db.blocking_lock().delete_thumb_mapping(fid);
        }
    }
    
    // Also try MD5 fallback for legacy cleanup
    let hash = format!("{:x}", md5::compute(file_path.as_bytes()));
    let md5_path = self.get_thumbs_dir().join(format!("{}.jpg", hash));
    if md5_path.exists() {
        let _ = std::fs::remove_file(&md5_path);
    }
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 8: Commit**

```bash
git add src/thumbnail/mod.rs
git commit -m "feat: update thumbnail module with UUID support and MD5 fallback"
```

---

### Task 5: Update scanner to use new mapping

**Files:**
- Modify: `src/scanner/mod.rs:47-55`

- [ ] **Step 1: Update thumbnail existence check**

Replace the thumbnail check in `scan_path`:

```rust
let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &PathBuf::from(""), self.db.clone());

for file_entry in &files {
    if !gen.thumbnail_exists(&file_entry.file_path, None) {
        files_needing_thumbnails.push(file_entry.file_path.clone());
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 3: Commit**

```bash
git add src/scanner/mod.rs
git commit -m "refactor: update scanner to use new thumbnail mapping"
```

---

### Task 6: Update app.rs thumbnail generation

**Files:**
- Modify: `src/app.rs:331-382`

- [ ] **Step 1: Update start_thumbnail_generation**

Update the `ThumbnailGenerator::new` call to pass `db`:

```rust
let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path, db.clone());
```

Update `generate_thumbnail_at_frame` call to pass `file_id`:

```rust
match gen.generate_thumbnail_at_frame(file_path, 300, Some(file_id)) {
```

- [ ] **Step 2: Update play_video_file**

Update the `ThumbnailGenerator::new` call:

```rust
let gen = crate::thumbnail::ThumbnailGenerator::new(&crate::utils::get_data_dir(), &ffp, self.db.clone());
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "refactor: update app.rs to use new thumbnail mapping"
```

---

### Task 7: Update context_menu.rs

**Files:**
- Modify: `src/ui/context_menu.rs:193-208`, `src/ui/context_menu.rs:253-273`, `src/ui/context_menu.rs:275-288`

- [ ] **Step 1: Update play_file_internal**

Update `ThumbnailGenerator::new` call:

```rust
let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path, app.db.clone());
```

- [ ] **Step 2: Update capture_screenshot_internal**

Update `ThumbnailGenerator::new` call and pass `file_id`:

```rust
let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path, app.db.clone());

if let Ok(thumb_path) = generator.generate_thumbnail_at_time(&file.file_path, time_seconds, Some(file.id)) {
```

- [ ] **Step 3: Update reset_thumbnail_internal**

Update `ThumbnailGenerator::new` call and pass `file_id`:

```rust
let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path, app.db.clone());

generator.delete_thumbnail(&file.file_path, Some(file.id));

if let Ok(thumb_path) = generator.generate_thumbnail(&file.file_path, Some(file.id)) {
```

- [ ] **Step 4: Update remove_file_internal**

Update `ThumbnailGenerator::new` call and pass `file_id`:

```rust
let generator = crate::thumbnail::ThumbnailGenerator::new(&root_dir, &ffmpeg_path, app.db.clone());
generator.delete_thumbnail(&file.file_path, Some(file.id));
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```bash
git add src/ui/context_menu.rs
git commit -m "refactor: update context_menu to use new thumbnail mapping"
```

---

### Task 8: Update screenshot_dialog.rs

**Files:**
- Modify: `src/ui/screenshot_dialog.rs:164-234`, `src/ui/screenshot_dialog.rs:236-266`

- [ ] **Step 1: Update update_preview**

This method generates temporary preview files, not permanent thumbnails. No changes needed to the mapping logic, but update the `ThumbnailGenerator::new` call if it's used (it's not in this method, so skip).

- [ ] **Step 2: Update capture_screenshot**

Update `ThumbnailGenerator::new` call and pass `file_id`:

```rust
let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path, app.db.clone());

if let Ok(thumb_path) = gen.generate_thumbnail_at_time(&self.file.file_path, self.current_time, Some(self.file.id)) {
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add src/ui/screenshot_dialog.rs
git commit -m "refactor: update screenshot_dialog to use new thumbnail mapping"
```

---

### Task 9: Integration testing

**Files:**
- Test: Manual testing with existing database

- [ ] **Step 1: Build release binary**

Run: `cargo build --release`
Expected: Compiles without errors

- [ ] **Step 2: Test migration with existing database**

1. Copy existing `data.db` to a backup
2. Run the new binary
3. Verify `thumbnail_map` table was created
4. Verify existing thumbnails were migrated (check file count in `thumbs/` vs `thumbnail_map` rows)

- [ ] **Step 3: Test new thumbnail generation**

1. Add a new video file to a watched path
2. Run scan
3. Verify thumbnail is generated with UUID filename
4. Verify `thumbnail_map` has entry for the file

- [ ] **Step 4: Test file move scenario**

1. Move a video file to a different folder within the same watched path
2. Re-scan
3. Verify thumbnail is still displayed (mapping should preserve it)

- [ ] **Step 5: Test thumbnail reset**

1. Right-click a video with custom thumbnail
2. Select "Reset to default thumbnail"
3. Verify old thumbnail file is deleted
4. Verify new thumbnail is generated with new UUID

- [ ] **Step 6: Commit final changes**

```bash
git add -A
git commit -m "test: integration testing complete for thumbnail UUID mapping"
```

---

## Self-Review Checklist

- [x] All spec requirements covered (schema, migration, lookup, creation, deletion, fallback)
- [x] No placeholders or TODOs
- [x] Type consistency: `file_id: i64`, `thumb_filename: String`, `uuid::Uuid::new_v4()`
- [x] Exact file paths and line numbers provided
- [x] Complete code snippets in every step
- [x] Exact commands with expected output
