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

    pub fn remove_path(&mut self, path_id: i64) -> SqliteResult<()> {
        info!("Removing watched path: {}", path_id);
        let tx = self.conn.transaction()?;

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

    pub fn upsert_files_batch(&self, files: &[FileEntry]) -> SqliteResult<()> {
        if files.is_empty() {
            return Ok(());
        }

        const BATCH_SIZE: usize = 500;

        for chunk in files.chunks(BATCH_SIZE) {
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
                params.push(Box::new(file.modified_at.clone()));
                params.push(Box::new(file.rating));
                params.push(Box::new(file.has_custom_thumb));
            }

            sql.push_str(" ON CONFLICT(path_id, file_path) DO UPDATE SET file_name = excluded.file_name, thumbnail_path = COALESCE(excluded.thumbnail_path, files.thumbnail_path), file_size = excluded.file_size, modified_at = excluded.modified_at");

            let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            self.conn.execute(&sql, params_ref.as_slice())?;
        }

        Ok(())
    }

    pub fn get_files_for_path(&self, path_id: i64) -> SqliteResult<Vec<FileEntry>> {
        let thumbs_dir = crate::utils::get_data_dir().join("thumbs");
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.path_id, f.file_path, f.file_name, f.file_ext,
                    tm.thumb_filename,
                    f.file_size, f.modified_at, f.rating, f.has_custom_thumb, f.created_at
             FROM files f
             LEFT JOIN thumbnail_map tm ON tm.file_id = f.id
             WHERE f.path_id = ?1",
        )?;
        let files = stmt
            .query_map([path_id], |row| {
                let thumb_filename: Option<String> = row.get(5)?;
                let thumbnail_path = thumb_filename
                    .map(|f| thumbs_dir.join(f).to_string_lossy().to_string());
                Ok(FileEntry {
                    id: row.get(0)?,
                    path_id: row.get(1)?,
                    file_path: row.get(2)?,
                    file_name: row.get(3)?,
                    file_ext: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    thumbnail_path,
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
        let thumbs_dir = crate::utils::get_data_dir().join("thumbs");
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.path_id, f.file_path, f.file_name, f.file_ext,
                    tm.thumb_filename,
                    f.file_size, f.modified_at, f.rating, f.has_custom_thumb, f.created_at
             FROM files f
             LEFT JOIN thumbnail_map tm ON tm.file_id = f.id
             WHERE f.id = ?1",
        )?;
        let result = stmt.query_row([file_id], |row| {
            let thumb_filename: Option<String> = row.get(5)?;
            let thumbnail_path = thumb_filename
                .map(|f| thumbs_dir.join(f).to_string_lossy().to_string());
            Ok(FileEntry {
                id: row.get(0)?,
                path_id: row.get(1)?,
                file_path: row.get(2)?,
                file_name: row.get(3)?,
                file_ext: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                thumbnail_path,
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

// ============ Thumbnails ============

impl Database {
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

    pub fn set_thumb_filename(&self, file_id: i64, filename: &str) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO thumbnail_map (file_id, thumb_filename) VALUES (?1, ?2)",
            params![file_id, filename],
        )?;
        Ok(())
    }

    pub fn delete_thumb_mapping(&self, file_id: i64) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM thumbnail_map WHERE file_id = ?1",
            [file_id],
        )?;
        Ok(())
    }

    pub fn get_all_thumb_filenames(&self) -> SqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT thumb_filename FROM thumbnail_map"
        )?;
        let filenames = stmt
            .query_map([], |row| row.get(0))?
            .collect::<SqliteResult<Vec<_>>>()?;
        Ok(filenames)
    }

    pub fn get_all_legacy_thumb_filenames(&self) -> SqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT thumbnail_path FROM files WHERE thumbnail_path IS NOT NULL"
        )?;
        let filenames = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                Ok(std::path::PathBuf::from(&path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or(path))
            })?
            .collect::<SqliteResult<Vec<_>>>()?;
        Ok(filenames)
    }

    pub fn delete_orphan_thumb_mappings(&self) -> SqliteResult<usize> {
        let thumbs_dir = crate::utils::get_data_dir().join("thumbs");
        let mut stmt = self.conn.prepare(
            "SELECT file_id, thumb_filename FROM thumbnail_map"
        )?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<SqliteResult<Vec<_>>>()?;

        let mut deleted = 0;
        for (file_id, filename) in rows {
            let path = thumbs_dir.join(&filename);
            if !path.exists() {
                self.conn.execute(
                    "DELETE FROM thumbnail_map WHERE file_id = ?1",
                    [file_id],
                )?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::new_in_memory().expect("Failed to create in-memory database")
    }

    fn test_file(path_id: i64, file_path: &str) -> FileEntry {
        let name = std::path::Path::new(file_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = std::path::Path::new(file_path)
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        FileEntry {
            id: 0,
            path_id,
            file_path: file_path.to_string(),
            file_name: name,
            file_ext: ext,
            thumbnail_path: None,
            file_size: 1024,
            modified_at: "2025-01-01 00:00:00".to_string(),
            rating: 0,
            has_custom_thumb: false,
            created_at: String::new(),
        }
    }

    // ============ Watched Paths ============

    #[test]
    fn test_add_and_get_paths() {
        let db = test_db();
        let id1 = db.add_path("/videos/a", "Folder A").unwrap();
        let id2 = db.add_path("/videos/b", "Folder B").unwrap();
        assert!(id1 > 0);
        assert!(id2 > id1);

        let paths = db.get_all_paths().unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "/videos/a");
        assert_eq!(paths[0].display_name, "Folder A");
        assert_eq!(paths[1].path, "/videos/b");
    }

    #[test]
    fn test_add_duplicate_path_fails() {
        let db = test_db();
        db.add_path("/videos/a", "A").unwrap();
        assert!(db.add_path("/videos/a", "A duplicate").is_err());
    }

    #[test]
    fn test_path_exists() {
        let db = test_db();
        assert!(!db.path_exists("/videos/a").unwrap());
        db.add_path("/videos/a", "A").unwrap();
        assert!(db.path_exists("/videos/a").unwrap());
    }

    #[test]
    fn test_remove_path() {
        let mut db = test_db();
        let id = db.add_path("/videos/a", "A").unwrap();
        db.remove_path(id).unwrap();
        assert!(!db.path_exists("/videos/a").unwrap());
        assert!(db.get_all_paths().unwrap().is_empty());
    }

    #[test]
    fn test_remove_path_cascades_files() {
        let mut db = test_db();
        let path_id = db.add_path("/videos/a", "A").unwrap();
        db.upsert_file(&test_file(path_id, "/videos/a/test.mp4")).unwrap();
        db.remove_path(path_id).unwrap();
        assert!(db.get_files_for_path(path_id).unwrap().is_empty());
    }

    // ============ Files ============

    #[test]
    fn test_upsert_and_get_files() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();

        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        assert!(file_id > 0);

        let files = db.get_files_for_path(path_id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_path, "/videos/a.mp4");
        assert_eq!(files[0].file_name, "a.mp4");
        assert_eq!(files[0].file_ext, "mp4");
    }

    #[test]
    fn test_upsert_file_updates_existing() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();

        let mut file = test_file(path_id, "/videos/a.mp4");
        file.file_size = 100;
        db.upsert_file(&file).unwrap();

        file.file_size = 200;
        db.upsert_file(&file).unwrap();

        let files = db.get_files_for_path(path_id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_size, 200);
    }

    #[test]
    fn test_batch_upsert() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();

        let files = vec![
            test_file(path_id, "/videos/a.mp4"),
            test_file(path_id, "/videos/b.mp4"),
            test_file(path_id, "/videos/c.mp4"),
        ];

        db.upsert_files_batch(&files).unwrap();

        let result = db.get_files_for_path(path_id).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_batch_upsert_updates_existing() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();

        let mut files = vec![test_file(path_id, "/videos/a.mp4")];
        files[0].file_size = 100;
        db.upsert_files_batch(&files).unwrap();

        files[0].file_size = 200;
        db.upsert_files_batch(&files).unwrap();

        let result = db.get_files_for_path(path_id).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_size, 200);
    }

    #[test]
    fn test_get_file_by_id() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();

        let file = db.get_file_by_id(file_id).unwrap().unwrap();
        assert_eq!(file.file_path, "/videos/a.mp4");

        assert!(db.get_file_by_id(99999).unwrap().is_none());
    }

    #[test]
    fn test_update_file_rating() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();

        db.update_file_rating(file_id, 4).unwrap();
        let file = db.get_file_by_id(file_id).unwrap().unwrap();
        assert_eq!(file.rating, 4);
    }

    #[test]
    fn test_rating_clamped_to_valid_range() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();

        db.update_file_rating(file_id, 10).unwrap();
        assert_eq!(db.get_file_by_id(file_id).unwrap().unwrap().rating, 5);

        db.update_file_rating(file_id, -5).unwrap();
        assert_eq!(db.get_file_by_id(file_id).unwrap().unwrap().rating, 0);
    }

    #[test]
    fn test_update_thumbnail() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();

        db.update_thumbnail(file_id, "/thumbs/test.jpg").unwrap();
        let file = db.get_file_by_id(file_id).unwrap().unwrap();
        assert!(file.has_custom_thumb);
    }

    #[test]
    fn test_delete_file() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();

        db.delete_file(file_id).unwrap();
        assert!(db.get_file_by_id(file_id).unwrap().is_none());
    }

    #[test]
    fn test_move_file() {
        let db = test_db();
        let path_id1 = db.add_path("/videos/a", "A").unwrap();
        let path_id2 = db.add_path("/videos/b", "B").unwrap();
        let file_id = db.upsert_file(&test_file(path_id1, "/videos/a/test.mp4")).unwrap();

        db.move_file(file_id, path_id2, "/videos/b/test.mp4").unwrap();

        assert!(db.get_files_for_path(path_id1).unwrap().is_empty());
        let files = db.get_files_for_path(path_id2).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_path, "/videos/b/test.mp4");
    }

    // ============ Tags ============

    #[test]
    fn test_create_and_get_tags() {
        let db = test_db();
        let id = db.create_tag("Action", "#ff0000").unwrap();
        assert!(id > 0);

        let tags = db.get_all_tags().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Action");
        assert_eq!(tags[0].color, "#ff0000");
    }

    #[test]
    fn test_update_tag() {
        let db = test_db();
        let id = db.create_tag("Action", "#ff0000").unwrap();
        db.update_tag(id, "Comedy", "#00ff00").unwrap();

        let tags = db.get_all_tags().unwrap();
        assert_eq!(tags[0].name, "Comedy");
        assert_eq!(tags[0].color, "#00ff00");
    }

    #[test]
    fn test_delete_tag() {
        let db = test_db();
        let id = db.create_tag("Action", "#ff0000").unwrap();
        db.delete_tag(id).unwrap();
        assert!(db.get_all_tags().unwrap().is_empty());
    }

    #[test]
    fn test_duplicate_tag_name_fails() {
        let db = test_db();
        db.create_tag("Action", "#ff0000").unwrap();
        assert!(db.create_tag("Action", "#00ff00").is_err());
    }

    #[test]
    fn test_add_and_get_tags_for_file() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        let tag1 = db.create_tag("Action", "#ff0000").unwrap();
        let tag2 = db.create_tag("Comedy", "#00ff00").unwrap();

        db.add_tag_to_file(file_id, tag1).unwrap();
        db.add_tag_to_file(file_id, tag2).unwrap();

        let tags = db.get_tags_for_file(file_id).unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_add_duplicate_tag_to_file_is_idempotent() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        let tag_id = db.create_tag("Action", "#ff0000").unwrap();

        db.add_tag_to_file(file_id, tag_id).unwrap();
        db.add_tag_to_file(file_id, tag_id).unwrap();

        assert_eq!(db.get_tags_for_file(file_id).unwrap().len(), 1);
    }

    #[test]
    fn test_remove_tag_from_file() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        let tag_id = db.create_tag("Action", "#ff0000").unwrap();

        db.add_tag_to_file(file_id, tag_id).unwrap();
        db.remove_tag_from_file(file_id, tag_id).unwrap();
        assert!(db.get_tags_for_file(file_id).unwrap().is_empty());
    }

    #[test]
    fn test_get_all_file_tags() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file1 = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        let file2 = db.upsert_file(&test_file(path_id, "/videos/b.mp4")).unwrap();
        let tag = db.create_tag("Action", "#ff0000").unwrap();

        db.add_tag_to_file(file1, tag).unwrap();
        db.add_tag_to_file(file2, tag).unwrap();

        let map = db.get_all_file_tags(path_id).unwrap();
        assert_eq!(map.len(), 2);
        assert!(map[&file1].contains(&tag));
        assert!(map[&file2].contains(&tag));
    }

    #[test]
    fn test_delete_tag_cascades_file_tags() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        let tag_id = db.create_tag("Action", "#ff0000").unwrap();
        db.add_tag_to_file(file_id, tag_id).unwrap();

        db.delete_tag(tag_id).unwrap();
        assert!(db.get_tags_for_file(file_id).unwrap().is_empty());
    }

    // ============ Thumbnails ============

    #[test]
    fn test_set_and_get_thumb_filename() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();

        db.set_thumb_filename(file_id, "abc-123.jpg").unwrap();
        let filename = db.get_thumb_filename(file_id).unwrap();
        assert_eq!(filename, Some("abc-123.jpg".to_string()));
    }

    #[test]
    fn test_set_thumb_replaces_existing() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();

        db.set_thumb_filename(file_id, "old.jpg").unwrap();
        db.set_thumb_filename(file_id, "new.jpg").unwrap();
        assert_eq!(db.get_thumb_filename(file_id).unwrap(), Some("new.jpg".to_string()));
    }

    #[test]
    fn test_delete_thumb_mapping() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();

        db.set_thumb_filename(file_id, "test.jpg").unwrap();
        db.delete_thumb_mapping(file_id).unwrap();
        assert_eq!(db.get_thumb_filename(file_id).unwrap(), None);
    }

    #[test]
    fn test_get_all_thumb_filenames() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let f1 = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        let f2 = db.upsert_file(&test_file(path_id, "/videos/b.mp4")).unwrap();

        db.set_thumb_filename(f1, "thumb1.jpg").unwrap();
        db.set_thumb_filename(f2, "thumb2.jpg").unwrap();

        let mut filenames = db.get_all_thumb_filenames().unwrap();
        filenames.sort();
        assert_eq!(filenames, vec!["thumb1.jpg", "thumb2.jpg"]);
    }

    #[test]
    fn test_get_thumb_filename_nonexistent() {
        let db = test_db();
        assert_eq!(db.get_thumb_filename(99999).unwrap(), None);
    }

    // ============ Settings ============

    #[test]
    fn test_default_settings_exist() {
        let db = test_db();
        assert!(db.get_setting("video_extensions").unwrap().is_some());
        assert!(db.get_setting("thumb_size").unwrap().is_some());
        assert!(db.get_setting("default_player").unwrap().is_some());
    }

    #[test]
    fn test_set_and_get_setting() {
        let db = test_db();
        db.set_setting("custom_key", "custom_value").unwrap();
        assert_eq!(db.get_setting("custom_key").unwrap(), Some("custom_value".to_string()));
    }

    #[test]
    fn test_set_setting_overwrites() {
        let db = test_db();
        db.set_setting("key", "v1").unwrap();
        db.set_setting("key", "v2").unwrap();
        assert_eq!(db.get_setting("key").unwrap(), Some("v2".to_string()));
    }

    #[test]
    fn test_get_nonexistent_setting() {
        let db = test_db();
        assert_eq!(db.get_setting("nonexistent").unwrap(), None);
    }

    #[test]
    fn test_get_video_extensions() {
        let db = test_db();
        let exts = db.get_video_extensions();
        assert!(exts.contains(&"mp4".to_string()));
        assert!(exts.contains(&"mkv".to_string()));
        assert!(exts.contains(&"avi".to_string()));
    }

    // ============ Cascade / Integrity ============

    #[test]
    fn test_delete_file_cascades_file_tags() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        let tag_id = db.create_tag("Action", "#ff0000").unwrap();
        db.add_tag_to_file(file_id, tag_id).unwrap();

        db.delete_file(file_id).unwrap();
        assert!(db.get_tags_for_file(file_id).unwrap().is_empty());
    }

    #[test]
    fn test_delete_file_cascades_thumb_mapping() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let file_id = db.upsert_file(&test_file(path_id, "/videos/a.mp4")).unwrap();
        db.set_thumb_filename(file_id, "test.jpg").unwrap();

        db.delete_file(file_id).unwrap();
        assert_eq!(db.get_thumb_filename(file_id).unwrap(), None);
    }

    #[test]
    fn test_rating_constraint_via_upsert() {
        let db = test_db();
        let path_id = db.add_path("/videos", "V").unwrap();
        let mut file = test_file(path_id, "/videos/a.mp4");
        file.rating = 3;
        db.upsert_file(&file).unwrap();

        let files = db.get_files_for_path(path_id).unwrap();
        assert_eq!(files[0].rating, 3);
    }

    #[test]
    fn test_get_files_for_path_only_returns_matching_path() {
        let db = test_db();
        let p1 = db.add_path("/a", "A").unwrap();
        let p2 = db.add_path("/b", "B").unwrap();
        db.upsert_file(&test_file(p1, "/a/one.mp4")).unwrap();
        db.upsert_file(&test_file(p2, "/b/two.mp4")).unwrap();

        let files_a = db.get_files_for_path(p1).unwrap();
        let files_b = db.get_files_for_path(p2).unwrap();
        assert_eq!(files_a.len(), 1);
        assert_eq!(files_b.len(), 1);
        assert_eq!(files_a[0].file_path, "/a/one.mp4");
        assert_eq!(files_b[0].file_path, "/b/two.mp4");
    }
}
