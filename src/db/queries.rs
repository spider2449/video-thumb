use super::{Database, FileEntry, Tag, WatchedPath};
use log::info;
use rusqlite::{params, Result as SqliteResult};

// ============ Watched Paths ============

impl Database {
    pub fn add_path(&self, path: &str, display_name: &str) -> SqliteResult<i64> {
        info!("Adding watched path: {} ({})", path, display_name);
        self.conn.execute(
            "INSERT INTO watched_paths (path, display_name) VALUES (?1, ?2)",
            params![path, display_name],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn remove_path(&self, path_id: i64) -> SqliteResult<()> {
        info!("Removing watched path: {}", path_id);
        let tx = self.conn.unchecked_transaction()?;

        // Delete file_tag associations for files in this path
        tx.execute(
            "DELETE FROM file_tags WHERE file_id IN (SELECT id FROM files WHERE path_id = ?1)",
            [path_id],
        )?;

        // Delete files for this path
        tx.execute("DELETE FROM files WHERE path_id = ?1", [path_id])?;

        // Delete the watched path
        tx.execute("DELETE FROM watched_paths WHERE id = ?1", [path_id])?;

        tx.commit()?;
        Ok(())
    }

    pub fn get_all_paths(&self) -> SqliteResult<Vec<WatchedPath>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, display_name, created_at FROM watched_paths ORDER BY id")?;
        let paths = stmt
            .query_map([], |row| {
                Ok(WatchedPath {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    display_name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    created_at: row.get(3)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;
        Ok(paths)
    }

    pub fn path_exists(&self, path: &str) -> SqliteResult<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM watched_paths WHERE path = ?1")?;
        let count: i32 = stmt.query_row([path], |row| row.get(0))?;
        Ok(count > 0)
    }
}

// ============ Files ============

impl Database {
    pub fn upsert_file(&self, file: &FileEntry) -> SqliteResult<i64> {
        self.conn.execute(
            "INSERT INTO files (path_id, file_path, file_name, file_ext, thumbnail_path, file_size, modified_at, rating, has_custom_thumb)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(path_id, file_path) DO UPDATE SET
                file_name = excluded.file_name,
                thumbnail_path = COALESCE(excluded.thumbnail_path, files.thumbnail_path),
                file_size = excluded.file_size,
                modified_at = excluded.modified_at",
            params![
                file.path_id,
                file.file_path,
                file.file_name,
                file.file_ext,
                file.thumbnail_path,
                file.file_size,
                file.modified_at,
                file.rating,
                file.has_custom_thumb,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_files_for_path(&self, path_id: i64) -> SqliteResult<Vec<FileEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path_id, file_path, file_name, file_ext, thumbnail_path,
                    file_size, modified_at, rating, has_custom_thumb, created_at
             FROM files WHERE path_id = ?1",
        )?;
        let files = stmt
            .query_map([path_id], |row| {
                Ok(FileEntry {
                    id: row.get(0)?,
                    path_id: row.get(1)?,
                    file_path: row.get(2)?,
                    file_name: row.get(3)?,
                    file_ext: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    thumbnail_path: row.get(5)?,
                    file_size: row.get(6)?,
                    modified_at: row.get(7)?,
                    rating: row.get(8)?,
                    has_custom_thumb: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;
        Ok(files)
    }

    pub fn get_file_by_id(&self, file_id: i64) -> SqliteResult<Option<FileEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path_id, file_path, file_name, file_ext, thumbnail_path,
                    file_size, modified_at, rating, has_custom_thumb, created_at
             FROM files WHERE id = ?1",
        )?;
        let result = stmt.query_row([file_id], |row| {
            Ok(FileEntry {
                id: row.get(0)?,
                path_id: row.get(1)?,
                file_path: row.get(2)?,
                file_name: row.get(3)?,
                file_ext: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                thumbnail_path: row.get(5)?,
                file_size: row.get(6)?,
                modified_at: row.get(7)?,
                rating: row.get(8)?,
                has_custom_thumb: row.get(9)?,
                created_at: row.get(10)?,
            })
        });

        match result {
            Ok(file) => Ok(Some(file)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn update_file_rating(&self, file_id: i64, rating: i32) -> SqliteResult<()> {
        let rating = rating.clamp(0, 5);
        self.conn.execute(
            "UPDATE files SET rating = ?1 WHERE id = ?2",
            params![rating, file_id],
        )?;
        Ok(())
    }

    pub fn update_thumbnail(&self, file_id: i64, thumbnail_path: &str) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE files SET thumbnail_path = ?1, has_custom_thumb = TRUE WHERE id = ?2",
            params![thumbnail_path, file_id],
        )?;
        Ok(())
    }

    pub fn delete_file(&self, file_id: i64) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM files WHERE id = ?1", [file_id])?;
        Ok(())
    }

    pub fn move_file(&self, file_id: i64, new_path_id: i64, new_file_path: &str) -> SqliteResult<()> {
        info!("Moving file {} to path_id={}, new_path={}", file_id, new_path_id, new_file_path);
        self.conn.execute(
            "UPDATE files SET path_id = ?1, file_path = ?2 WHERE id = ?3",
            params![new_path_id, new_file_path, file_id],
        )?;
        Ok(())
    }
}

// ============ Tags ============

impl Database {
    pub fn create_tag(&self, name: &str, color: &str) -> SqliteResult<i64> {
        info!("Creating tag: {} ({})", name, color);
        self.conn.execute(
            "INSERT INTO tags (name, color) VALUES (?1, ?2)",
            params![name, color],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_tag(&self, tag_id: i64, name: &str, color: &str) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE tags SET name = ?1, color = ?2 WHERE id = ?3",
            params![name, color, tag_id],
        )?;
        Ok(())
    }

    pub fn delete_tag(&self, tag_id: i64) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM tags WHERE id = ?1", [tag_id])?;
        Ok(())
    }

    pub fn get_all_tags(&self) -> SqliteResult<Vec<Tag>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, color FROM tags ORDER BY name")?;
        let tags = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;
        Ok(tags)
    }

    pub fn add_tag_to_file(&self, file_id: i64, tag_id: i64) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            params![file_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_tag_from_file(&self, file_id: i64, tag_id: i64) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM file_tags WHERE file_id = ?1 AND tag_id = ?2",
            params![file_id, tag_id],
        )?;
        Ok(())
    }

    pub fn get_tags_for_file(&self, file_id: i64) -> SqliteResult<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.color FROM tags t
             INNER JOIN file_tags ft ON ft.tag_id = t.id
             WHERE ft.file_id = ?1 ORDER BY t.name",
        )?;
        let tags = stmt
            .query_map([file_id], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;
        Ok(tags)
    }

    pub fn get_all_file_tags(&self, path_id: i64) -> SqliteResult<std::collections::HashMap<i64, Vec<i64>>> {
        let mut stmt = self.conn.prepare(
            "SELECT ft.file_id, ft.tag_id FROM file_tags ft
             INNER JOIN files f ON f.id = ft.file_id
             WHERE f.path_id = ?1",
        )?;
        let mut map: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
        let mut rows = stmt.query([path_id])?;
        while let Some(row) = rows.next()? {
            let file_id: i64 = row.get(0)?;
            let tag_id: i64 = row.get(1)?;
            map.entry(file_id).or_default().push(tag_id);
        }
        Ok(map)
    }

    pub fn tag_name_exists(&self, name: &str) -> SqliteResult<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM tags WHERE name = ?1")?;
        let count: i32 = stmt.query_row([name], |row| row.get(0))?;
        Ok(count > 0)
    }
}