# Thumbnail Features Design

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to implement this design task-by-task.

**Goal:** Implement frame 300 thumbnail generation and timeline-based screenshot capture with preview.

**Architecture:** Frame-based thumbnail extraction with ffmpeg, timeline dialog for custom frame selection with on-demand frame preview.

**Tech Stack:** Rust, egui 0.31, ffmpeg (ffprobe for frame counting)

---

## Feature B: Frame 300 Thumbnail Generation

### Requirements
- When adding movies, generate thumbnail at frame 300 instead of 1 second
- Check if video has at least 300 frames using ffprobe
- If video is shorter than 300 frames, fall back to frame 1 and log a warning
- No UI changes - this is a backend behavior change

### Design

**ThumbsGenerator changes:**
Add new method `generate_thumbnail_at_frame(video_path, frame_number)`:
1. Use ffprobe to count frames or attempt extraction
2. Use ffmpeg `-vf "select=eq(n\,299)"` to extract specific frame
3. If extraction fails (video too short), fall back to frame 0
4. Return result with path

**Scanner changes:**
In `scan_path()`, after creating FileEntry:
1. Call `generate_thumbnail_at_frame(&file_entry.file_path, 300)`
2. If returns Ok, update database with thumbnail path
3. If returns Err, log warning and try frame 1 as fallback

### Interface

```rust
// In src/thumbnail/mod.rs
impl ThumbnailGenerator {
    /// Generate thumbnail at specific frame, falls back to frame 1 if video too short
    pub fn generate_thumbnail_at_frame(&self, video_path: &str, frame: u32) -> Result<String, String> {
        // Try frame 300 first
        // If fails, try frame 1
        // Return path to thumbnail
    }
}
```

---

## Feature C: Timeline Screenshot Dialog

### Requirements
- New dialog for capturing custom thumbnails with timeline scrub
- Opens from context menu "截圖" screenshot option
- 320x180 preview area showing current frame
- Timeline slider (0 to total duration/frames)
- Shows current time/frame position
- "Capture" button to save current frame as thumbnail
- "Cancel" button to close without saving
- No auto-play - static image preview

### Design

**UI Structure:**

```
+----------------------------------+
|  截圖 - 選擇時間點               |
+----------------------------------+
|                                  |
|    +----------------------+     |
|    |                      |     |
|    |   Video Preview      |     |
|    |   320x180             |     |
|    |                      |     |
|    +----------------------+     |
|                                  |
|  |<========O===========>|  <-- Timeline slider
|                                  |
|  時間: 00:05:30  (幀: 8290)      |
|                                  |
|    [取消]    [截圖]              |
+----------------------------------+
```

**Implementation:**

1. **New Dialog Struct** (`src/ui/screenshot_dialog.rs`):
   - `file: FileEntry` - video being captured
   - `current_time: f64` - current position in seconds
   - `total_duration: f64` - video total duration (from ffprobe)
   - `preview_texture: Option<TextureHandle>` - cached preview image
   - `is_loading: bool` - flag while extracting frame

2. **Frame Extraction:**
   - When slider moves, debounce 100ms
   - Extract frame at current position via ffmpeg
   - Load into egui::Image/TextureHandle
   - Update preview

3. **Capture:**
   - Save current preview frame as custom thumbnail
   - Update database
   - Close dialog

### Interface

```rust
// In src/ui/screenshot_dialog.rs
pub struct ScreenshotDialog {
    pub file: FileEntry,
    pub current_time: f64,
    pub total_duration: f64,
    pub preview_texture: Option<egui::TextureHandle>,
    pub is_loading: bool,
}

impl ScreenshotDialog {
    pub fn new(file: FileEntry) -> Self { ... }
    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) -> bool { ... }
    fn update_preview(&mut self, ctx: &egui::Context) { ... }
}
```

**Context Menu Integration:**
- Replace text input + button with "📷 截圖..." button
- When clicked: set `app.screenshot_dialog = Some(ScreenshotDialog::new(file))`

---

## File Changes

| Feature | File | Changes |
|---------|------|---------|
| B | `src/thumbnail/mod.rs` | Add `generate_thumbnail_at_frame()` method |
| B | `src/scanner/mod.rs` | Call new method, handle fallback |
| C | `src/ui/screenshot_dialog.rs` | New file - timeline dialog |
| C | `src/ui/context_menu.rs` | Replace time input with "截圖..." button |
| C | `src/ui/mod.rs` | Export ScreenshotDialog |
| C | `src/app.rs` | Add screenshot_dialog field |
| Both | `src/ui/main_window.rs` | Render screenshot dialog |

---

## Sequence of Implementation

1. Feature B: Add frame-based extraction to thumbnail module
2. Feature B: Update scanner to use frame 300
3. Create thumbnail dialog struct and basic UI
4. Add preview extraction logic
5. Integrate into context menu
6. Test both features end-to-end

---

## Testing

**Feature B:**
- Add video with 300+ frames - thumbnail at frame 300
- Add video with < 300 frames - falls back, logs warning
- Verify thumbnail displays correctly

**Feature C:**
- Right-click video -> "截圖..." opens dialog
- Preview shows initial frame
- Drag slider updates preview within 100ms
- Click "截圖" saves custom thumbnail
- Verify `has_custom_thumb` indicator shows

---

## Spec Self-Review

- [x] No placeholders
- [x] Frame fallback logic clear
- [x] Dialog dimensions specified (320x180)
- [x] No auto-play confirmed
- [x] File structure complete
