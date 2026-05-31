mod models;
mod queries;

use log::info;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;

pub use models::*;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &Path) -> SqliteResult<Self> {
        info!("Initializing database at: {:?}", path);
        let conn = Connection::open(path)?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;

        let db = Self { conn };
        db.init_schema()?;
        db.init_default_settings()?;
        db.migrate_thumbnails()?;

        Ok(db)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let db = Self { conn };
        db.init_schema()?;
        db.init_default_settings()?;
        Ok(db)
    }

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
            ",
        )?;
        Ok(())
    }

    fn init_default_settings(&self) -> SqliteResult<()> {
        let defaults = vec![
            ("default_player", ""),
            (
                "video_extensions",
                "mp4,mkv,avi,mov,wmv,flv,webm,m4v,mpg,mpeg,3gp,ogv,ts,m2ts,vob,rm,rmvb",
            ),
            ("thumb_size", "256"),
        ];

        for (key, value) in defaults {
            self.conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                [key, value],
            )?;
        }
        Ok(())
    }

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

    pub fn get_video_extensions(&self) -> Vec<String> {
        match self.get_setting("video_extensions") {
            Ok(Some(exts)) => exts
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
            _ => vec![],
        }
    }

    pub fn get_setting(&self, key: &str) -> SqliteResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let result: SqliteResult<String> = stmt.query_row([key], |row| row.get(0));
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            [key, value],
        )?;
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        info!("Closing database connection");
    }
}