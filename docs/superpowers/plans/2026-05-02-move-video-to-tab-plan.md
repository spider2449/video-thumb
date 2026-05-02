# Move Video to Another Tab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add "移到其他標籤頁" (Move to Tab) in the context menu that physically moves video files and updates database records.

**Architecture:** Add a new database query for moving files and implement UI dropdown in context menu. The move operation copies the file to the new location then deletes the original (handles cross-drive moves), then updates the database.

**Tech Stack:** Rust, egui, rusqlite

---

## File Structure

| File | Change |
|------|--------|
| `src/db/queries.rs` | Add `move_file` query |
| `src/ui/context_menu.rs` | Add move dropdown and `move_file_to_tab_internal` method |

---

## Task 1: Add Database Query for Moving Files

**Files:**
- Modify: `src/db/queries.rs` - Add after `delete_file`

- [ ] **Step 1: Add `move_file` method to Database impl**

In `src/db/queries.rs`, after line 166 (after `delete_file` function), add:

```rust
pub fn move_file(&self, file_id: i64, new_path_id: i64, new_file_path: &str) -> SqliteResult<()> {
    info!("Moving file {} to path_id={}, new_path={}", file_id, new_path_id, new_file_path);
    self.conn.execute(
        "UPDATE files SET path_id = ?1, file_path = ?2 WHERE id = ?3",
        params![new_path_id, new_file_path, file_id],
    )?;
    Ok(())
}
```

- [ ] **Step 2: Run cargo check to verify**

Run: `cargo check`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src/db/queries.rs
git commit -m "feat(db): add move_file query for relocating videos to different tabs"
```

---

## Task 2: Add Move to Tab UI in Context Menu

**Files:**
- Modify: `src/ui/context_menu.rs` - Add dropdown menu and move logic

- [ ] **Step 1: Add `move_file_to_tab_internal` method in ContextMenu impl**

In `src/ui/context_menu.rs`, after line 293 (before `hex_to_color` function), add:

```rust
fn move_file_to_tab_internal(&mut self, file: &FileEntry, new_path_id: i64, new_file_path: &str, app: &mut App) {
    let source_path = std::path::PathBuf::from(&file.file_path);
    let dest_path = std::path::PathBuf::from(new_file_path);

    // Try direct rename first (same drive)
    let move_result = std::fs::rename(&source_path, &dest_path);

    // If cross-drive, copy then delete
    let move_result = move_result.or_else(|_| {
        std::fs::copy(&source_path, &dest_path)
            .and_then(|_| std::fs::remove_file(&source_path))
    });

    match move_result {
        Ok(_) => {
            // Update database
            app.db.blocking_lock()
                .move_file(file.id, new_path_id, new_file_path)
                .ok();

            // Refresh files in current tab (remove the moved file)
            if let Some(path_id) = app.current_path_id {
                if let Ok(files) = app.db.blocking_lock().get_files_for_path(path_id) {
                    app.files.insert(path_id, files);
                }
            }

            // Refresh destination tab if it's loaded
            if let Ok(files) = app.db.blocking_lock().get_files_for_path(new_path_id) {
                app.files.insert(new_path_id, files);
            }

            self.file = None;
        }
        Err(e) => {
            log::error!("Failed to move file: {}", e);
        }
    }
}
```

- [ ] **Step 2: Add Move to Tab dropdown in the context menu UI**

In `src/ui/context_menu.rs`, after line 117 (after Screenshot button, before Open Folder), add:

```rust
ui.separator();

// Move to tab submenu
let other_tabs: Vec<_> = app.watched_paths.iter()
    .filter(|p| p.id != file.path_id)
    .collect();

if !other_tabs.is_empty() {
    ui.collapsing("➡️ 移到其他標籤頁", |ui| {
        for dest_tab in &other_tabs {
            let dest_tab = dest_tab.clone();
            if ui.button(&dest_tab.display_name).clicked() {
                // Compute new file path in destination directory
                let new_path = std::path::PathBuf::from(&dest_tab.path)
                    .join(&file.file_name);
                let new_path_str = new_path.to_string_lossy().to_string();
                self.move_file_to_tab_internal(&file, dest_tab.id, &new_path_str, app);
            }
        }
    });
} else {
    ui.label("沒有其他標籤頁");
}
```

- [ ] **Step 3: Run cargo check to verify**

Run: `cargo check`
Expected: No errors

- [ ] **Step 4: Test build**

Run: `cargo build 2>&1 | head -50`
Expected: BUILD SUCCESSFUL

- [ ] **Step 5: Commit**

```bash
git add src/ui/context_menu.rs
git commit -m "feat(ui): add Move to Tab option in context menu"
```

---

## Verification Steps

1. Run `cargo build --release` to verify it compiles
2. Start the app and test:
   - Right-click a video
   - Hover "移到其他標籤頁" to see the dropdown
   - Select a different tab
   - Verify file was moved on disk and in database