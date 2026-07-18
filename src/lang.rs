#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

use once_cell::sync::Lazy;
use std::collections::HashMap;

static TRANSLATIONS: Lazy<HashMap<&'static str, HashMap<Lang, &'static str>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("add_path", [(Lang::ZhTw, "+ 新增路徑"), (Lang::En, "+ Add Path")].into_iter().collect());
    m.insert("scanning_prefix", [(Lang::ZhTw, "掃描中... "), (Lang::En, "Scanning... ")].into_iter().collect());
    m.insert("scanning_suffix", [(Lang::ZhTw, "%"), (Lang::En, "%")].into_iter().collect());
    m.insert("update_db", [(Lang::ZhTw, "🔄 更新資料庫"), (Lang::En, "🔄 Update Database")].into_iter().collect());
    m.insert("rescan_thumbs", [(Lang::ZhTw, "🗑 清理縮圖"), (Lang::En, "🗑 Rescan thumbs")].into_iter().collect());
    m.insert("sort", [(Lang::ZhTw, "排序"), (Lang::En, "Sort")].into_iter().collect());
    m.insert("name", [(Lang::ZhTw, "名稱"), (Lang::En, "Name")].into_iter().collect());
    m.insert("date", [(Lang::ZhTw, "日期"), (Lang::En, "Date")].into_iter().collect());
    m.insert("rating", [(Lang::ZhTw, "評分"), (Lang::En, "Rating")].into_iter().collect());
    m.insert("search", [(Lang::ZhTw, "搜尋:"), (Lang::En, "Search:")].into_iter().collect());
    m.insert("rating_filter", [(Lang::ZhTw, "評分篩選"), (Lang::En, "Rating Filter")].into_iter().collect());
    m.insert("all", [(Lang::ZhTw, "全部"), (Lang::En, "All")].into_iter().collect());
    m.insert("stars_suffix", [(Lang::ZhTw, " 星"), (Lang::En, " Stars")].into_iter().collect());
    m.insert("tag_filter", [(Lang::ZhTw, "標籤篩選"), (Lang::En, "Tag Filter")].into_iter().collect());
    m.insert("tag_manager", [(Lang::ZhTw, "🏷️ 標籤管理"), (Lang::En, "🏷️ Tag Manager")].into_iter().collect());
    m.insert("size", [(Lang::ZhTw, "大小"), (Lang::En, "Size")].into_iter().collect());
    m.insert("settings", [(Lang::ZhTw, "⚙️ 設定"), (Lang::En, "⚙️ Settings")].into_iter().collect());
    m.insert("confirm_remove_path", [(Lang::ZhTw, "確認移除路徑"), (Lang::En, "Confirm Remove Path")].into_iter().collect());
    m.insert("confirm_remove_prefix", [(Lang::ZhTw, "確定要移除「"), (Lang::En, "Are you sure you want to remove \"")].into_iter().collect());
    m.insert("confirm_remove_suffix", [(Lang::ZhTw, "」嗎？"), (Lang::En, "\"?")].into_iter().collect());
    m.insert("remove_only_records", [(Lang::ZhTw, "（只會移除記錄，不會刪除檔案）"), (Lang::En, "(Only removes records, files are not deleted)")].into_iter().collect());
    m.insert("confirm_remove", [(Lang::ZhTw, "確認移除"), (Lang::En, "Confirm Remove")].into_iter().collect());
    m.insert("cancel", [(Lang::ZhTw, "取消"), (Lang::En, "Cancel")].into_iter().collect());
    m.insert("file_count_prefix", [(Lang::ZhTw, "共 "), (Lang::En, "")].into_iter().collect());
    m.insert("file_count_suffix", [(Lang::ZhTw, " 個檔案"), (Lang::En, " files total")].into_iter().collect());
    m.insert("add_path_title", [(Lang::ZhTw, "新增監控路徑"), (Lang::En, "Add Watched Path")].into_iter().collect());
    m.insert("folder_path", [(Lang::ZhTw, "資料夾路徑:"), (Lang::En, "Folder Path:")].into_iter().collect());
    m.insert("browse", [(Lang::ZhTw, "選擇..."), (Lang::En, "Browse...")].into_iter().collect());
    m.insert("display_name", [(Lang::ZhTw, "顯示名稱:"), (Lang::En, "Display Name:")].into_iter().collect());
    m.insert("add", [(Lang::ZhTw, "新增"), (Lang::En, "Add")].into_iter().collect());
    m.insert("thumbs_dir_not_exist", [(Lang::ZhTw, "縮圖資料夾不存在"), (Lang::En, "Thumbnail directory does not exist")].into_iter().collect());
    m.insert("cannot_read_thumbs_dir_prefix", [(Lang::ZhTw, "無法讀取縮圖資料夾: "), (Lang::En, "Cannot read thumbnail directory: ")].into_iter().collect());
    m.insert("cleaned_thumbs_prefix", [(Lang::ZhTw, "已刪除 "), (Lang::En, "Deleted ")].into_iter().collect());
    m.insert("cleaned_thumbs_mid", [(Lang::ZhTw, " 個未使用的縮圖，清理 "), (Lang::En, " unused thumbnails, cleaned ")].into_iter().collect());
    m.insert("cleaned_thumbs_suffix", [(Lang::ZhTw, " 筆無效紀錄"), (Lang::En, " orphan records")].into_iter().collect());
    m.insert("scanning", [(Lang::ZhTw, "掃描中..."), (Lang::En, "Scanning...")].into_iter().collect());
    m.insert("play", [(Lang::ZhTw, "▶ 播放"), (Lang::En, "▶ Play")].into_iter().collect());
    m.insert("rating_label", [(Lang::ZhTw, "評分:"), (Lang::En, "Rating:")].into_iter().collect());
    m.insert("tags_label", [(Lang::ZhTw, "標籤:"), (Lang::En, "Tags:")].into_iter().collect());
    m.insert("add_tag", [(Lang::ZhTw, "新增:"), (Lang::En, "Add:")].into_iter().collect());
    m.insert("select_tag", [(Lang::ZhTw, "選擇標籤"), (Lang::En, "Select Tag")].into_iter().collect());
    m.insert("manage_tags", [(Lang::ZhTw, "🏷️ 管理標籤..."), (Lang::En, "🏷️ Manage Tags...")].into_iter().collect());
    m.insert("screenshot", [(Lang::ZhTw, "📷 截圖..."), (Lang::En, "📷 Screenshot...")].into_iter().collect());
    m.insert("move_to_tab", [(Lang::ZhTw, "➡️ 移到其他標籤頁"), (Lang::En, "➡️ Move to Other Tab")].into_iter().collect());
    m.insert("no_other_tabs", [(Lang::ZhTw, "沒有其他標籤頁"), (Lang::En, "No other tabs")].into_iter().collect());
    m.insert("open_folder", [(Lang::ZhTw, "📂 開啟所在資料夾"), (Lang::En, "📂 Open Containing Folder")].into_iter().collect());
    m.insert("copy_path", [(Lang::ZhTw, "📋 複製路徑"), (Lang::En, "📋 Copy Path")].into_iter().collect());
    m.insert("reset_thumb", [(Lang::ZhTw, "🔄 恢復預設縮圖"), (Lang::En, "🔄 Reset Thumbnail")].into_iter().collect());
    m.insert("remove_record", [(Lang::ZhTw, "❌ 移除記錄"), (Lang::En, "❌ Remove Record")].into_iter().collect());
    m.insert("settings_title", [(Lang::ZhTw, "設定"), (Lang::En, "Settings")].into_iter().collect());
    m.insert("player_path", [(Lang::ZhTw, "播放器路徑:"), (Lang::En, "Player Path:")].into_iter().collect());
    m.insert("executable", [(Lang::ZhTw, "執行檔"), (Lang::En, "Executable")].into_iter().collect());
    m.insert("player_hint", [(Lang::ZhTw, "留空使用系統預設播放器"), (Lang::En, "Leave empty to use system default player")].into_iter().collect());
    m.insert("thumb_size_label", [(Lang::ZhTw, "縮圖大小:"), (Lang::En, "Thumbnail Size:")].into_iter().collect());
    m.insert("pixels", [(Lang::ZhTw, "像素"), (Lang::En, "px")].into_iter().collect());
    m.insert("cache_dir", [(Lang::ZhTw, "快取目錄:"), (Lang::En, "Cache Directory:")].into_iter().collect());
    m.insert("cache_dir_hint", [(Lang::ZhTw, "留空使用系統預設目錄 (需重新啟動生效)"), (Lang::En, "Leave empty for default (requires restart)")].into_iter().collect());
    m.insert("language", [(Lang::ZhTw, "語言:"), (Lang::En, "Language:")].into_iter().collect());
    m.insert("close", [(Lang::ZhTw, "關閉"), (Lang::En, "Close")].into_iter().collect());
    m.insert("save", [(Lang::ZhTw, "儲存"), (Lang::En, "Save")].into_iter().collect());
    m.insert("save_player_failed_prefix", [(Lang::ZhTw, "儲存播放器路徑失敗: "), (Lang::En, "Failed to save player path: ")].into_iter().collect());
    m.insert("invalid_number", [(Lang::ZhTw, "請輸入有效數字"), (Lang::En, "Please enter a valid number")].into_iter().collect());
    m.insert("thumb_size_range", [(Lang::ZhTw, "縮圖大小必須在 64-1024 之間"), (Lang::En, "Thumbnail size must be between 64-1024")].into_iter().collect());
    m.insert("save_thumb_size_failed_prefix", [(Lang::ZhTw, "儲存縮圖大小失敗: "), (Lang::En, "Failed to save thumbnail size: ")].into_iter().collect());
    m.insert("save_cache_dir_failed_prefix", [(Lang::ZhTw, "儲存快取目錄失敗: "), (Lang::En, "Failed to save cache directory: ")].into_iter().collect());
    m.insert("tag_manager_title", [(Lang::ZhTw, "標籤管理"), (Lang::En, "Tag Manager")].into_iter().collect());
    m.insert("existing_tags", [(Lang::ZhTw, "現有標籤"), (Lang::En, "Existing Tags")].into_iter().collect());
    m.insert("delete", [(Lang::ZhTw, "刪除"), (Lang::En, "Delete")].into_iter().collect());
    m.insert("edit", [(Lang::ZhTw, "編輯"), (Lang::En, "Edit")].into_iter().collect());
    m.insert("edit_tag", [(Lang::ZhTw, "編輯標籤:"), (Lang::En, "Edit Tag:")].into_iter().collect());
    m.insert("color", [(Lang::ZhTw, "顏色:"), (Lang::En, "Color:")].into_iter().collect());
    m.insert("new_tag", [(Lang::ZhTw, "新增標籤:"), (Lang::En, "New Tag:")].into_iter().collect());
    m.insert("tag_name_empty", [(Lang::ZhTw, "標籤名稱不能為空"), (Lang::En, "Tag name cannot be empty")].into_iter().collect());
    m.insert("create_tag_failed_prefix", [(Lang::ZhTw, "建立標籤失敗: "), (Lang::En, "Failed to create tag: ")].into_iter().collect());
    m.insert("screenshot_title", [(Lang::ZhTw, "截圖 - 選擇時間點"), (Lang::En, "Screenshot - Select Time")].into_iter().collect());
    m.insert("loading", [(Lang::ZhTw, "載入中..."), (Lang::En, "Loading...")].into_iter().collect());
    m.insert("drag_timeline", [(Lang::ZhTw, "拖動時間軸預覽"), (Lang::En, "Drag timeline to preview")].into_iter().collect());
    m.insert("time_prefix", [(Lang::ZhTw, "時間: "), (Lang::En, "Time: ")].into_iter().collect());
    m.insert("timeline", [(Lang::ZhTw, "時間軸"), (Lang::En, "Timeline")].into_iter().collect());
    m.insert("current_prefix", [(Lang::ZhTw, "當前: "), (Lang::En, "Current: ")].into_iter().collect());
    m.insert("total_prefix", [(Lang::ZhTw, "總長: "), (Lang::En, "Total: ")].into_iter().collect());
    m.insert("capture", [(Lang::ZhTw, "截圖"), (Lang::En, "Capture")].into_iter().collect());
    m.insert("select_folder_path", [(Lang::ZhTw, "請選擇資料夾路徑"), (Lang::En, "Please select a folder path")].into_iter().collect());
    m.insert("path_not_exist", [(Lang::ZhTw, "路徑不存在"), (Lang::En, "Path does not exist")].into_iter().collect());
    m.insert("path_not_dir", [(Lang::ZhTw, "路徑不是有效的資料夾"), (Lang::En, "Path is not a valid folder")].into_iter().collect());
    m.insert("path_already_exists", [(Lang::ZhTw, "此路徑已存在"), (Lang::En, "This path already exists")].into_iter().collect());
    m.insert("add_failed_prefix", [(Lang::ZhTw, "新增失敗: "), (Lang::En, "Failed to add: ")].into_iter().collect());
    m.insert("query_failed_prefix", [(Lang::ZhTw, "查詢失敗: "), (Lang::En, "Query failed: ")].into_iter().collect());
    m.insert("generating_thumbs_prefix", [(Lang::ZhTw, "生成縮圖中... "), (Lang::En, "Generating thumbnails... ")].into_iter().collect());
    m.insert("generating_thumbs_sep", [(Lang::ZhTw, "/"), (Lang::En, "/")].into_iter().collect());
    m.insert("thumbs_done", [(Lang::ZhTw, "縮圖生成完成"), (Lang::En, "Thumbnail generation complete")].into_iter().collect());
    m.insert("done_found_prefix", [(Lang::ZhTw, "完成！找到 "), (Lang::En, "Done! Found ")].into_iter().collect());
    m.insert("done_found_suffix", [(Lang::ZhTw, " 個檔案"), (Lang::En, " files")].into_iter().collect());
    m.insert("failed_prefix", [(Lang::ZhTw, "失敗: "), (Lang::En, "Failed: ")].into_iter().collect());
    m.insert("unknown", [(Lang::En, "unknown")].into_iter().collect());
    m
});

pub fn t(lang: Lang, key: &str) -> &'static str {
    let Some(map) = TRANSLATIONS.get(key) else { return "" };
    map.get(&lang)
        .or_else(|| map.get(&Lang::En))
        .copied()
        .unwrap_or("")
}
