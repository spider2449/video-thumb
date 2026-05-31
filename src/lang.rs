#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    ZhTw,
    En,
}

impl Lang {
    pub fn from_setting(s: &str) -> Self {
        match s {
            "en" => Lang::En,
            _ => Lang::ZhTw,
        }
    }

    pub fn as_setting(&self) -> &'static str {
        match self {
            Lang::ZhTw => "zh_tw",
            Lang::En => "en",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Lang::ZhTw => "繁體中文",
            Lang::En => "English",
        }
    }

    pub fn all() -> &'static [Lang] {
        &[Lang::ZhTw, Lang::En]
    }
}

pub fn t(lang: Lang, key: &str) -> &'static str {
    match (lang, key) {
        (Lang::ZhTw, "add_path") => "+ 新增路徑",
        (_, "add_path") => "+ Add Path",

        (Lang::ZhTw, "scanning_prefix") => "掃描中... ",
        (_, "scanning_prefix") => "Scanning... ",

        (Lang::ZhTw, "scanning_suffix") => "%",
        (_, "scanning_suffix") => "%",

        (Lang::ZhTw, "update_db") => "🔄 更新資料庫",
        (_, "update_db") => "🔄 Update Database",

        (Lang::ZhTw, "rescan_thumbs") => "🗑 清理縮圖",
        (_, "rescan_thumbs") => "🗑 Rescan thumbs",

        (Lang::ZhTw, "sort") => "排序",
        (_, "sort") => "Sort",

        (Lang::ZhTw, "name") => "名稱",
        (_, "name") => "Name",

        (Lang::ZhTw, "date") => "日期",
        (_, "date") => "Date",

        (Lang::ZhTw, "rating") => "評分",
        (_, "rating") => "Rating",

        (Lang::ZhTw, "search") => "搜尋:",
        (_, "search") => "Search:",

        (Lang::ZhTw, "rating_filter") => "評分篩選",
        (_, "rating_filter") => "Rating Filter",

        (Lang::ZhTw, "all") => "全部",
        (_, "all") => "All",

        (Lang::ZhTw, "stars_suffix") => " 星",
        (_, "stars_suffix") => " Stars",

        (Lang::ZhTw, "tag_filter") => "標籤篩選",
        (_, "tag_filter") => "Tag Filter",

        (Lang::ZhTw, "tag_manager") => "🏷️ 標籤管理",
        (_, "tag_manager") => "🏷️ Tag Manager",

        (Lang::ZhTw, "size") => "大小",
        (_, "size") => "Size",

        (Lang::ZhTw, "settings") => "⚙️ 設定",
        (_, "settings") => "⚙️ Settings",

        (Lang::ZhTw, "confirm_remove_path") => "確認移除路徑",
        (_, "confirm_remove_path") => "Confirm Remove Path",

        (Lang::ZhTw, "confirm_remove_prefix") => "確定要移除「",
        (_, "confirm_remove_prefix") => "Are you sure you want to remove \"",

        (Lang::ZhTw, "confirm_remove_suffix") => "」嗎？",
        (_, "confirm_remove_suffix") => "\"?",

        (Lang::ZhTw, "remove_only_records") => "（只會移除記錄，不會刪除檔案）",
        (_, "remove_only_records") => "(Only removes records, files are not deleted)",

        (Lang::ZhTw, "confirm_remove") => "確認移除",
        (_, "confirm_remove") => "Confirm Remove",

        (Lang::ZhTw, "cancel") => "取消",
        (_, "cancel") => "Cancel",

        (Lang::ZhTw, "file_count_prefix") => "共 ",
        (_, "file_count_prefix") => "",

        (Lang::ZhTw, "file_count_suffix") => " 個檔案",
        (_, "file_count_suffix") => " files total",

        (Lang::ZhTw, "add_path_title") => "新增監控路徑",
        (_, "add_path_title") => "Add Watched Path",

        (Lang::ZhTw, "folder_path") => "資料夾路徑:",
        (_, "folder_path") => "Folder Path:",

        (Lang::ZhTw, "browse") => "選擇...",
        (_, "browse") => "Browse...",

        (Lang::ZhTw, "display_name") => "顯示名稱:",
        (_, "display_name") => "Display Name:",

        (Lang::ZhTw, "add") => "新增",
        (_, "add") => "Add",

        (Lang::ZhTw, "thumbs_dir_not_exist") => "縮圖資料夾不存在",
        (_, "thumbs_dir_not_exist") => "Thumbnail directory does not exist",

        (Lang::ZhTw, "cannot_read_thumbs_dir_prefix") => "無法讀取縮圖資料夾: ",
        (_, "cannot_read_thumbs_dir_prefix") => "Cannot read thumbnail directory: ",

        (Lang::ZhTw, "cleaned_thumbs_prefix") => "已刪除 ",
        (_, "cleaned_thumbs_prefix") => "Deleted ",

        (Lang::ZhTw, "cleaned_thumbs_mid") => " 個未使用的縮圖，清理 ",
        (_, "cleaned_thumbs_mid") => " unused thumbnails, cleaned ",

        (Lang::ZhTw, "cleaned_thumbs_suffix") => " 筆無效紀錄",
        (_, "cleaned_thumbs_suffix") => " orphan records",

        (Lang::ZhTw, "scanning") => "掃描中...",
        (_, "scanning") => "Scanning...",

        (Lang::ZhTw, "play") => "▶ 播放",
        (_, "play") => "▶ Play",

        (Lang::ZhTw, "rating_label") => "評分:",
        (_, "rating_label") => "Rating:",

        (Lang::ZhTw, "tags_label") => "標籤:",
        (_, "tags_label") => "Tags:",

        (Lang::ZhTw, "add_tag") => "新增:",
        (_, "add_tag") => "Add:",

        (Lang::ZhTw, "select_tag") => "選擇標籤",
        (_, "select_tag") => "Select Tag",

        (Lang::ZhTw, "manage_tags") => "🏷️ 管理標籤...",
        (_, "manage_tags") => "🏷️ Manage Tags...",

        (Lang::ZhTw, "screenshot") => "📷 截圖...",
        (_, "screenshot") => "📷 Screenshot...",

        (Lang::ZhTw, "move_to_tab") => "➡️ 移到其他標籤頁",
        (_, "move_to_tab") => "➡️ Move to Other Tab",

        (Lang::ZhTw, "no_other_tabs") => "沒有其他標籤頁",
        (_, "no_other_tabs") => "No other tabs",

        (Lang::ZhTw, "open_folder") => "📂 開啟所在資料夾",
        (_, "open_folder") => "📂 Open Containing Folder",

        (Lang::ZhTw, "copy_path") => "📋 複製路徑",
        (_, "copy_path") => "📋 Copy Path",

        (Lang::ZhTw, "reset_thumb") => "🔄 恢復預設縮圖",
        (_, "reset_thumb") => "🔄 Reset Thumbnail",

        (Lang::ZhTw, "remove_record") => "❌ 移除記錄",
        (_, "remove_record") => "❌ Remove Record",

        (Lang::ZhTw, "settings_title") => "設定",
        (_, "settings_title") => "Settings",

        (Lang::ZhTw, "player_path") => "播放器路徑:",
        (_, "player_path") => "Player Path:",

        (Lang::ZhTw, "executable") => "執行檔",
        (_, "executable") => "Executable",

        (Lang::ZhTw, "player_hint") => "留空使用系統預設播放器",
        (_, "player_hint") => "Leave empty to use system default player",

        (Lang::ZhTw, "thumb_size_label") => "縮圖大小:",
        (_, "thumb_size_label") => "Thumbnail Size:",

        (Lang::ZhTw, "pixels") => "像素",
        (_, "pixels") => "px",

        (Lang::ZhTw, "cache_dir") => "快取目錄:",
        (_, "cache_dir") => "Cache Directory:",

        (Lang::ZhTw, "cache_dir_hint") => "留空使用系統預設目錄 (需重新啟動生效)",
        (_, "cache_dir_hint") => "Leave empty for default (requires restart)",

        (Lang::ZhTw, "language") => "語言:",
        (_, "language") => "Language:",

        (Lang::ZhTw, "close") => "關閉",
        (_, "close") => "Close",

        (Lang::ZhTw, "save") => "儲存",
        (_, "save") => "Save",

        (Lang::ZhTw, "save_player_failed_prefix") => "儲存播放器路徑失敗: ",
        (_, "save_player_failed_prefix") => "Failed to save player path: ",

        (Lang::ZhTw, "invalid_number") => "請輸入有效數字",
        (_, "invalid_number") => "Please enter a valid number",

        (Lang::ZhTw, "thumb_size_range") => "縮圖大小必須在 64-1024 之間",
        (_, "thumb_size_range") => "Thumbnail size must be between 64-1024",

        (Lang::ZhTw, "save_thumb_size_failed_prefix") => "儲存縮圖大小失敗: ",
        (_, "save_thumb_size_failed_prefix") => "Failed to save thumbnail size: ",

        (Lang::ZhTw, "save_cache_dir_failed_prefix") => "儲存快取目錄失敗: ",
        (_, "save_cache_dir_failed_prefix") => "Failed to save cache directory: ",

        (Lang::ZhTw, "tag_manager_title") => "標籤管理",
        (_, "tag_manager_title") => "Tag Manager",

        (Lang::ZhTw, "existing_tags") => "現有標籤",
        (_, "existing_tags") => "Existing Tags",

        (Lang::ZhTw, "delete") => "刪除",
        (_, "delete") => "Delete",

        (Lang::ZhTw, "edit") => "編輯",
        (_, "edit") => "Edit",

        (Lang::ZhTw, "edit_tag") => "編輯標籤:",
        (_, "edit_tag") => "Edit Tag:",

        (Lang::ZhTw, "color") => "顏色:",
        (_, "color") => "Color:",

        (Lang::ZhTw, "new_tag") => "新增標籤:",
        (_, "new_tag") => "New Tag:",

        (Lang::ZhTw, "tag_name_empty") => "標籤名稱不能為空",
        (_, "tag_name_empty") => "Tag name cannot be empty",

        (Lang::ZhTw, "create_tag_failed_prefix") => "建立標籤失敗: ",
        (_, "create_tag_failed_prefix") => "Failed to create tag: ",

        (Lang::ZhTw, "screenshot_title") => "截圖 - 選擇時間點",
        (_, "screenshot_title") => "Screenshot - Select Time",

        (Lang::ZhTw, "loading") => "載入中...",
        (_, "loading") => "Loading...",

        (Lang::ZhTw, "drag_timeline") => "拖動時間軸預覽",
        (_, "drag_timeline") => "Drag timeline to preview",

        (Lang::ZhTw, "time_prefix") => "時間: ",
        (_, "time_prefix") => "Time: ",

        (Lang::ZhTw, "timeline") => "時間軸",
        (_, "timeline") => "Timeline",

        (Lang::ZhTw, "current_prefix") => "當前: ",
        (_, "current_prefix") => "Current: ",

        (Lang::ZhTw, "total_prefix") => "總長: ",
        (_, "total_prefix") => "Total: ",

        (Lang::ZhTw, "capture") => "截圖",
        (_, "capture") => "Capture",

        (Lang::ZhTw, "select_folder_path") => "請選擇資料夾路徑",
        (_, "select_folder_path") => "Please select a folder path",

        (Lang::ZhTw, "path_not_exist") => "路徑不存在",
        (_, "path_not_exist") => "Path does not exist",

        (Lang::ZhTw, "path_not_dir") => "路徑不是有效的資料夾",
        (_, "path_not_dir") => "Path is not a valid folder",

        (Lang::ZhTw, "path_already_exists") => "此路徑已存在",
        (_, "path_already_exists") => "This path already exists",

        (Lang::ZhTw, "add_failed_prefix") => "新增失敗: ",
        (_, "add_failed_prefix") => "Failed to add: ",

        (Lang::ZhTw, "query_failed_prefix") => "查詢失敗: ",
        (_, "query_failed_prefix") => "Query failed: ",

        (Lang::ZhTw, "generating_thumbs_prefix") => "生成縮圖中... ",
        (_, "generating_thumbs_prefix") => "Generating thumbnails... ",

        (Lang::ZhTw, "generating_thumbs_sep") => "/",
        (_, "generating_thumbs_sep") => "/",

        (Lang::ZhTw, "done_found_prefix") => "完成！找到 ",
        (_, "done_found_prefix") => "Done! Found ",

        (Lang::ZhTw, "done_found_suffix") => " 個檔案",
        (_, "done_found_suffix") => " files",

        (Lang::ZhTw, "failed_prefix") => "失敗: ",
        (_, "failed_prefix") => "Failed: ",

        (_, "unknown") => "unknown",
        _ => "",
    }
}
