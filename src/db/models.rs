#[derive(Debug, Clone)]
pub struct WatchedPath {
    pub id: i64,
    pub path: String,
    pub display_name: String,
    #[allow(dead_code)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub id: i64,
    pub path_id: i64,
    pub file_path: String,
    pub file_name: String,
    pub file_ext: String,
    pub thumbnail_path: Option<String>,
    pub file_size: i64,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub rating: i32,
    pub has_custom_thumb: bool,
    #[allow(dead_code)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.color)
    }
}