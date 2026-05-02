# Move Video to Another Tab - Design

**Date:** 2026-05-02
**Status:** Approved

## Overview

Add "移到其他標籤頁" (Move to Tab) option in the video context menu that physically moves the video file and updates its database record.

## UI Flow

1. Right-click on a video → context menu appears
2. "移到其他標籤頁" menu item shows a dropdown listing all other watched paths (tabs)
3. User selects destination tab
4. File is moved, database is updated, video disappears from current tab and appears in destination

## Implementation Details

### Database Changes

Add `move_file(file_id, new_path_id)` query in `src/db/queries.rs`:

```rust
pub fn move_file(&self, file_id: i64, new_path_id: i64, new_file_path: &str) -> SqliteResult<()> {
    self.conn.execute(
        "UPDATE files SET path_id = ?1, file_path = ?2 WHERE id = ?3",
        params![new_path_id, new_file_path, file_id],
    )?;
    Ok(())
}
```

### File Move Logic

In `src/ui/context_menu.rs`, add:

```rust
fn move_file_to_tab_internal(&self, file: &FileEntry, new_path_id: i64, new_file_path: &str, app: &mut App) {
    // 1. Move file physically
    let result = std::fs::rename(&file.file_path, &new_file_path);

    // 2. If cross-drive move failed, try copy+delete
    let result = result.or_else(|_| {
        std::fs::copy(&file.file_path, &new_file_path)
            .and_then(|_| std::fs::remove_file(&file.file_path))
    });

    match result {
        Ok(_) => {
            // 3. Update database
            app.db.blocking_lock()
                .move_file(file.id, new_path_id, &new_file_path)
                .ok();

            // 4. Refresh current tab files
            if let Some(path_id) = app.current_path_id {
                app.files.insert(path_id, app.db.blocking_lock().get_files_for_path(path_id).unwrap_or_default());
            }
        }
        Err(e) => {
            // Show error - will add status message
        }
    }
}
```

### UI Changes

In context menu (`src/ui/context_menu.rs`), add dropdown menu:

```rust
// In the popup menu, add:
ui.collapsing("➡️ 移到其他標籤頁", |ui| {
    for path in app.watched_paths.iter() {
        if path.id != file.path_id {
            let dest_path = path.clone();
            if ui.button(&path.display_name).clicked() {
                // Compute new file path
                let new_path = PathBuf::from(&dest_path.path)
                    .join(Path::new(&file.file_name));
                self.move_file_to_tab_internal(&file, dest_path.id, &new_path.to_string_lossy(), app);
                self.file = None;
            }
        }
    }
});
```

## Edge Cases

| Case | Handling |
|------|----------|
| Cross-drive move | Copy then delete original |
| File locked/in use | Show error message |
| Destination file exists | Append numeric suffix (e.g., `video_1.mp4`) |
| Move fails after copy | Keep copy, show error, don't delete original |

## Files to Modify

1. `src/db/queries.rs` - Add `move_file` query
2. `src/ui/context_menu.rs` - Add move menu item and logic
3. `src/app.rs` - May need refresh logic

## Testing Checklist

- [ ] Move file within same drive (rename succeeds)
- [ ] Move file across drives (copy+delete succeeds)
- [ ] File record updates correctly in database
- [ ] File list refreshes in UI after move
- [ ] Error handling shows proper message on failure