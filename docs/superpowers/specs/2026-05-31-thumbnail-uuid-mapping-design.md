# Thumbnail UUID Mapping Design

**Date:** 2026-05-31
**Status:** Draft
**Author:** Staff Engineer Review

## Problem

Thumbnails are currently named using MD5 hash of the full file path (`md5(file_path).jpg`). This breaks when:
- Files are moved within the same library (folder reorganization)
- The entire watched path is relocated to a different drive
- File paths change for any reason (case sensitivity, mount point changes)

Result: orphaned thumbnail files on disk, wasted storage, poor user experience.

## Solution

Replace MD5-based naming with UUID-based naming backed by a SQLite mapping table.

### Schema Change

```sql
CREATE TABLE IF NOT EXISTS thumbnail_map (
    file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    thumb_filename TEXT NOT NULL
);
```

- `file_id`: Links to `files.id`, cascade delete ensures cleanup
- `thumb_filename`: Stores UUID filename like `a1b2c3d4-e5f6-7890-abcd-ef1234567890.jpg`

### Legacy Column Handling

- `files.thumbnail_path` is deprecated but NOT removed
- New code reads/writes only `thumbnail_map`
- Column kept for safety; future migration can drop it

### Migration Flow (Hybrid)

On database initialization:

1. Create `thumbnail_map` table if it doesn't exist
2. For each file with non-null `files.thumbnail_path`:
   - Extract MD5 filename from the stored path
   - If the thumbnail file exists on disk:
     - Generate UUID v4
     - Rename file from `{md5}.jpg` to `{uuid}.jpg`
     - Insert `(file_id, "{uuid}.jpg")` into `thumbnail_map`
   - If the thumbnail file does NOT exist:
     - Do NOT create a mapping entry
     - Set `files.thumbnail_path` to NULL
     - Thumbnail will regenerate on next scan
3. Set `files.thumbnail_path` to NULL for migrated rows

### Thumbnail Lookup Flow (Hybrid Fallback)

When resolving a thumbnail for a file:

1. Query `thumbnail_map` for `file_id` → if found, return `{thumbs_dir}/{thumb_filename}`
2. Fallback: compute `md5(file_path)`, check if `{md5}.jpg` exists on disk
   - If yes: generate UUID, rename file, create mapping entry, return new path
   - This handles unmigrated thumbnails or edge cases
3. If neither exists, no thumbnail — return None

### Thumbnail Creation Flow

When generating a new thumbnail:

1. Generate UUID v4
2. Run ffmpeg to create thumbnail at `{thumbs_dir}/{uuid}.jpg`
3. Insert `(file_id, "{uuid}.jpg")` into `thumbnail_map` (INSERT OR REPLACE)

### Thumbnail Deletion Flow

When deleting a thumbnail:

1. Query `thumbnail_map` for `file_id`
2. If found, delete the file from disk and remove the mapping row
3. Fallback: also try deleting `{md5(file_path)}.jpg` for legacy cleanup

## Affected Files

| File | Change |
|------|--------|
| `src/db/mod.rs` | Add `thumbnail_map` table to schema, add migration logic in `init_schema` |
| `src/db/queries.rs` | Add `get_thumb_filename(file_id)`, `set_thumb_filename(file_id, uuid)`, `delete_thumb_mapping(file_id)` |
| `src/thumbnail/mod.rs` | Replace `get_thumbnail_path` to use UUID + mapping; add MD5 fallback; add UUID generation |
| `src/app.rs` | Update `start_thumbnail_generation` to use new mapping |
| `src/scanner/mod.rs` | Update thumbnail existence check to use mapping |
| `src/ui/context_menu.rs` | Update `reset_thumbnail_internal`, `capture_screenshot_internal` to use new mapping |
| `src/ui/screenshot_dialog.rs` | Update `capture_screenshot` to use new mapping |

## Dependencies

- `uuid` crate (v1.x) with `v4` feature for UUID generation
- No breaking changes to existing thumbnail files — hybrid fallback ensures compatibility

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Migration fails mid-way | Each file migration is independent; partial migration is safe |
| UUID collision | Statistically impossible with UUID v4; no additional check needed |
| Orphaned old thumbnails | MD5 fallback gradually migrates them; manual cleanup script can be added later |
| Performance regression | Single-row lookup in `thumbnail_map` is O(1); same as before |

## Success Criteria

- Thumbnails survive file moves within the same library
- Thumbnails survive watched path relocation
- Existing MD5-named thumbnails continue to work via fallback
- No user-visible disruption during migration
- All thumbnail operations (create, read, delete) use the new mapping
