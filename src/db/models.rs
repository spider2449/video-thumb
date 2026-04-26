#[derive(Debug, Clone)]
pub struct WatchedPath {
    pub id: i64,
    pub path: String,
    pub display_name: String,
    pub created_at: String,
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
    pub modified_at: String,
    pub rating: i32,
    pub has_custom_thumb: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
}

impl Tag {
    pub fn new(name: &str, color: &str) -> Self {
        Self {
            id: 0,
            name: name.to_string(),
            color: color.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileWithTags {
    pub file: FileEntry,
    pub tags: Vec<Tag>,
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.color)
    }
}