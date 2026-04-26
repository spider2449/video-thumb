# Thumbnail Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement frame 300 thumbnail generation and timeline-based screenshot capture with preview.

**Architecture:** Add frame-based extraction with ffmpeg, create timeline dialog for custom frame selection with on-demand frame preview using egui.

**Tech Stack:** Rust, egui 0.31, ffmpeg, ffprobe

---

## File Structure

| File | Responsibility | Changes |
|------|----------------|---------|
| `src/thumbnail/mod.rs` | Frame-based thumbnail extraction | Add `generate_thumbnail_at_frame()` method |
| `src/scanner/mod.rs` | Video scanning | Use frame 300 for thumbnails, fallback handling |
| `src/ui/screenshot_dialog.rs` | Timeline dialog (NEW) | Create - preview, timeline slider, capture |
| `src/ui/context_menu.rs` | Context menu | Replace time input with "截圖..." button |
| `src/ui/mod.rs` | Module exports | Add ScreenshotDialog export |
| `src/app.rs` | App state | Add `screenshot_dialog` field |
| `src/ui/main_window.rs` | Main window | Render screenshot dialog |

---

### Task 1: Add Frame-Based Thumbnail Generation Method

**Files:**
- Modify: `src/thumbnail/mod.rs:36-99` (after generate_thumbnail)

**Prerequisites:** Read current `src/thumbnail/mod.rs` to understand existing methods

- [ ] **Step 1: Add ffprobe helper function**

Add a helper method to get frame count using ffprobe:

```rust
// Add to impl ThumbnailGenerator, after generate_thumbnail_at_time

/// Get frame count using ffprobe
fn get_frame_count(&self, video_path: &str) -> Result<u32, String> {
    let output = Command::new(&self.ffmpeg_path)
        .args(["-v", "error"])
        .args(["-select_streams", "v:0"])
        .args(["-show_entries", "stream=nb_frames"])
        .args(["-of", "default=nokey=1:noprint_wrappers=1"])
        .arg(video_path)
        .output()
        .map_err(|e| format!("ffprobe failed: {}", e))?;
    
    let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if count_str.is_empty() || count_str == "N/A" {
        // Fallback: estimate from duration and fps
        return Err("Frame count not available".to_string());
    }
    
    count_str.parse::<u32>()
        .map_err(|e| format!("Failed to parse frame count: {}", e))
}
```

- [ ] **Step 2: Add generate_thumbnail_at_frame method**

```rust
/// Generate thumbnail at specific frame, fall back to frame 1 if video too short
pub fn generate_thumbnail_at_frame(&self, video_path: &str, target_frame: u32) -> Result<String, String> {
    let thumb_path = self.get_thumbnail_path(video_path);
    
    if thumb_path.exists() {
        info!("Thumbnail already exists: {:?}", thumb_path);
        return Ok(thumb_path.to_string_lossy().to_string());
    }
    
    std::fs::create_dir_all(self.get_thumbs_dir())
        .map_err(|e| format!("Failed to create thumbs directory: {}", e))?;
    
    // First try to get frame count
    let frame_to_use = match self.get_frame_count(video_path) {
        Ok(count) => {
            if count > target_frame {
                target_frame
            } else {
                log::warn!("Video has only {} frames, using frame 1 instead of {}", count, target_frame);
                1
            }
        }
        Err(e) => {
            log::warn!("Could not get frame count ({}), will try frame {} directly", e, target_frame);
            target_frame
        }
    };
    
    info!("Generating thumbnail at frame {} for: {}", frame_to_use, video_path);
    
    let output = Command::new(&self.ffmpeg_path)
        .args(["-y", "-i", video_path])
        .args(["-vf", &format!("select=eq(n\\,{}),scale={}:{}", frame_to_use - 1, self.width, self.height)])
        .args(["-vframes", "1"])
        .args(["-q:v", "2"])
        .arg(&thumb_path)
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Try frame 1 as fallback
        if frame_to_use != 1 {
            log::warn!("Frame {} failed, trying frame 1", frame_to_use);
            return self.generate_thumbnail_at_frame(video_path, 1);
        }
        return Err(format!("ffmpeg failed: {}", stderr));
    }
    
    info!("Thumbnail generated: {:?}", thumb_path);
    Ok(thumb_path.to_string_lossy().to_string())
}
```

- [ ] **Step 3: Build and verify no errors**

```bash
cargo check 2>&1 | grep -E "^error" | head -10
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/thumbnail/mod.rs
git commit -m "feat: add generate_thumbnail_at_frame method with fallback"
```

---

### Task 2: Update Scanner to Use Frame 300 Thumbnails

**Files:**
- Modify: `src/scanner/mod.rs`

- [ ] **Step 1: Read current scanner implementation**

Read `src/scanner/mod.rs` to see where thumbnails are generated.

- [ ] **Step 2: Update scan_path to use frame-based extraction**

```rust
// In scan_path method, after creating file_entry:

// Generate thumbnail at frame 300
let data_dir = crate::app::get_data_dir();
if let Some(ffmpeg_path) = Self::get_ffmpeg() {
    let generator = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path);
    match generator.generate_thumbnail_at_frame(&file_entry.file_path, 300) {
        Ok(thumb_path) => {
            // Update thumbnail path in file_entry
            file_entry.thumbnail_path = Some(thumb_path);
        }
        Err(e) => {
            log::error!("Failed to generate thumbnail for {}: {}", file_entry.file_path, e);
        }
    }
}

// Then insert into database as before
```

Actually, looking at scanner - it currently doesn't generate thumbnails during scan. Let me check where thumbnails are generated...

Looking at `ui/context_menu.rs`, thumbnails are generated via `reset_thumbnail_internal`. The initial scan just adds file entries without thumbnails.

Let me update the approach - actually add thumbnail generation during the scan:

```rust
// In create_file_entry, after creating the entry, generate thumbnail
// Add this to the end of create_file_entry:

// Generate thumbnail at frame 300
fn generate_thumbnail_for_entry(&self, file_entry: &mut FileEntry) {
    let data_dir = crate::app::get_data_dir();
    if let Some(ffmpeg_path) = Self::get_ffmpeg() {
        let generator = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path);
        match generator.generate_thumbnail_at_frame(&file_entry.file_path, 300) {
            Ok(thumb_path) => {
                file_entry.thumbnail_path = Some(thumb_path);
            }
            Err(e) => {
                log::warn!("Could not generate thumbnail for {}: {}", file_entry.file_path, e);
            }
        }
    }
}
```

And call it in create_file_entry before returning.

- [ ] **Step 3: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -10
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/scanner/mod.rs
git commit -m "feat: scanner generates thumbnails at frame 300"
```

---

### Task 3: Create ScreenshotDialog Module

**Files:**
- Create: `src/ui/screenshot_dialog.rs`

- [ ] **Step 1: Create screenshot_dialog.rs with full implementation**

```rust
use crate::app::App;
use crate::db::FileEntry;
use eframe::egui;
use std::path::PathBuf;

pub struct ScreenshotDialog {
    pub file: FileEntry,
    pub current_time: f64,
    pub total_duration: f64,
    pub preview_texture: Option<egui::TextureHandle>,
    pub is_loading: bool,
    pub thumbnail_size: [u32; 2],
}

impl ScreenshotDialog {
    pub fn new(file: FileEntry) -> Self {
        // Get video duration using ffmpeg
        let total_duration = Self::get_video_duration(&file.file_path).unwrap_or(60.0);
        
        // Start at 1 second (or 5 seconds if video is long enough)
        let start_time = if total_duration > 5.0 { 5.0 } else { 1.0 };
        
        Self {
            file,
            current_time: start_time,
            total_duration,
            preview_texture: None,
            is_loading: false,
            thumbnail_size: [320, 180],
        }
    }
    
    fn get_video_duration(video_path: &str) -> Option<f64> {
        // Use ffprobe to get duration
        let ffmpeg_path = Self::get_ffmpeg()?;
        
        let output = std::process::Command::new(&ffmpeg_path)
            .args(["-v", "error"])
            .args(["-show_entries", "format=duration"])
            .args(["-of", "default=noprint_wrappers=1:nokey=1"])
            .arg(video_path)
            .output()
            .ok()?;
        
        let duration_str = String::from_utf8_lossy(&output.stdout);
        duration_str.trim().parse::<f64>().ok()
    }
    
    fn get_ffmpeg() -> Option<PathBuf> {
        use dirs::data_local_dir;
        let root_dir = data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("video-thumb-viewer");
        let ffmpeg_dir = root_dir.join("ffmpeg");
        
        #[cfg(target_os = "windows")]
        {
            let path = ffmpeg_dir.join("ffmpeg.exe");
            if path.exists() { return Some(path); }
            if std::process::Command::new("where").arg("ffmpeg").output()
                .map(|o| o.status.success()).unwrap_or(false) {
                return Some(PathBuf::from("ffmpeg"));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let path = ffmpeg_dir.join("ffmpeg");
            if path.exists() { return Some(path); }
            return Some(PathBuf::from("ffmpeg"));
        }
        None
    }
    
    /// Returns true to keep dialog open
    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) -> bool {
        let mut open = true;
        
        egui::Window::new("截圖 - 選擇時間點")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(400.0);
                
                // Preview area
                ui.group(|ui| {
                    ui.set_min_height(200.0);
                    
                    // Show preview or loading indicator
                    if let Some(texture) = &self.preview_texture {
                        let size = egui::Vec2::new(
                            self.thumbnail_size[0] as f32,
                            self.thumbnail_size[1] as f32,
                        );
                        let img_rect = ui.available_rect_before_wrap();
                        // Center the image
                        let img_pos = img_rect.center() - (size / 2.0);
                        ui.put(
                            egui::Rect::from_min_size(img_pos, size),
                            egui::Image::from_texture(texture).fit_to_exact_size(size),
                        );
                    } else if self.is_loading {
                        ui.centered_and_justified(|ui| {
                            ui.label("載入中...");
                        });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("拖動時間軸預覽預覽");
                        });
                    }
                });
                
                ui.add_space(8.0);
                
                // Timeline slider
                let prev_time = self.current_time;
                ui.horizontal(|ui| {
                    ui.label("時間:");
                    ui.add(
                        egui::Slider::new(&mut self.current_time, 0.0..=self.total_duration)
                            .show_value(false),
                    );
                });
                
                // Time display
                let mins = (self.current_time / 60.0) as u32;
                let secs = (self.current_time % 60.0) as u32;
                let frames = (self.current_time * 30.0) as u32; // Approximate 30fps
                ui.label(format!("{:02}:{:02} (約 {} 靺)", mins, secs, frames));
                
                // Update preview if time changed
                if (self.current_time - prev_time).abs() > 0.1 && !self.is_loading {
                    self.update_preview(ctx);
                }
                
                ui.add_space(16.0);
                
                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        open = false;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("截圖").clicked() {
                            self.capture_screenshot(app);
                            open = false;
                        }
                    });
                });
            });
        
        open
    }
    
    fn update_preview(&mut self, ctx: &egui::Context) {
        self.is_loading = true;
        
        let Some(ffmpeg_path) = Self::get_ffmpeg() else {
            log::error!("ffmpeg not found");
            self.is_loading = false;
            return;
        };
        
        let video_path = self.file.file_path.clone();
        let time = self.current_time;
        let size = self.thumbnail_size;
        
        // Generate preview at current time
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("video-thumb-viewer");
        
        let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path);
        
        // Create temp preview file
        let temp_path = data_dir.join(format!("preview_{}.jpg", self.file.id));
        
        let time_str = format!("{:02}:{:02}:{:02}",
            (time / 3600.0) as u32,
            ((time % 3600.0) / 60.0) as u32,
            (time % 60.0) as u32
        );
        
        match std::process::Command::new(&ffmpeg_path)
            .args(["-y", "-ss", &time_str])
            .args(["-i", &video_path])
            .args(["-vframes", "1"])
            .args(["-vf", &format!("scale={}:{}", size[0], size[1])])
            .args(["-q:v", "2"])
            .arg(&temp_path)
            .output()
        {
            Ok(output) => {
                if output.status.success() && temp_path.exists() {
                    // Load image and create texture
                    if let Ok(img) = image::open(&temp_path) {
                        let rgba = img.to_rgba8();
                        let colors: Vec<egui::Color32> = rgba.pixels()
                            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                            .collect();
                        let color_img = egui::ColorImage {
                            size: [rgba.width() as usize, rgba.height() as usize],
                            pixels: colors,
                        };
                        self.preview_texture = Some(ctx.load_texture(
                            "preview",
                            color_img,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    // Clean up temp file
                    let _ = std::fs::remove_file(&temp_path);
                }
            }
            Err(e) => {
                log::error!("Failed to generate preview: {}", e);
            }
        }
        
        self.is_loading = false;
    }
    
    fn capture_screenshot(&mut self, app: &mut App) {
        let Some(ffmpeg_path) = Self::get_ffmpeg() else {
            log::error!("ffmpeg not found");
            return;
        };
        
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("video-thumb-viewer");
        
        let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path);
        
        if let Ok(thumb_path) = gen.generate_thumbnail_at_time(&self.file.file_path, self.current_time) {
            // Update database
            app.db.blocking_lock()
                .update_thumbnail(self.file.id, &thumb_path)
                .ok();
            
            log::info!("Screenshot saved: {}", thumb_path);
        }
    }
}

impl Default for ScreenshotDialog {
    fn default() -> Self {
        // Can't actually default without file, but required for Default trait
        Self {
            file: FileEntry {
                id: 0, path_id: 0, file_path: String::new(), file_name: String::new(),
                file_ext: String::new(), thumbnail_path: None, file_size: 0,
                modified_at: String::new(), rating: 0, has_custom_thumb: false, created_at: String::new(),
            },
            current_time: 0.0,
            total_duration: 60.0,
            preview_texture: None,
            is_loading: false,
            thumbnail_size: [320, 180],
        }
    }
}
```

- [ ] **Step 3: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -10
```

Expected: No errors (may have warnings about unused code)

- [ ] **Step 4: Commit**

```bash
git add src/ui/screenshot_dialog.rs
git commit -m "feat: create ScreenshotDialog with timeline and preview"
```

---

### Task 4: Add ScreenshotDialog to UI Module

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add ScreenshotDialog to mod.rs**

```rust
// Add at top
mod screenshot_dialog;

// Add to exports
pub use screenshot_dialog::ScreenshotDialog;
```

- [ ] **Step 2: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -10
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/mod.rs
git commit -m "chore: export ScreenshotDialog from ui module"
```

---

### Task 5: Add ScreenshotDialog State to App

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add screenshot_dialog field to App struct**

```rust
// Add after context_menu
pub screenshot_dialog: Option<crate::ui::ScreenshotDialog>,
```

- [ ] **Step 2: Initialize screenshot_dialog in App::new**

```rust
// In App::new initialization
screenshot_dialog: None,
```

- [ ] **Step 3: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -10
```

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "chore: add screenshot_dialog field to App state"
```

---

### Task 6: Update Context Menu to Open Screenshot Dialog

**Files:**
- Modify: `src/ui/context_menu.rs`

- [ ] **Step 1: Replace time input section with simplified version**

Replace the current time input section (around line 96-101):

```rust
// OLD CODE (around line 96-101):
ui.horizontal(|ui| {
    ui.label("時間:");
    ui.text_edit_singleline(&mut self.time_input);
    if ui.button("截圖").clicked() {
        self.capture_screenshot_internal(&file_for_capture, app);
    }
});

// NEW CODE:
ui.horizontal(|ui| {
    if ui.button("📷 截圖...").clicked() {
        app.screenshot_dialog = Some(crate::ui::ScreenshotDialog::new(file_for_capture));
        self.file = None; // Close context menu
    }
});
```

- [ ] **Step 2: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -10
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/context_menu.rs
git commit -m "feat: replace time input with ScreenshotDialog opener"
```

---

### Task 7: Render Screenshot Dialog in Main Window

**Files:**
- Modify: `src/ui/main_window.rs`

- [ ] **Step 1: Add dialog rendering after context menu section**

Find where context menu is rendered (around line 170-178), then add screenshot dialog after it:

```rust
    // Context menu overlay (rendered on top)
    {
        if let Some(mut menu) = app.context_menu.take() {
            menu.show(ctx, app);
            if menu.file.is_some() {
                app.context_menu = Some(menu);
            }
        }
    }
    
    // Screenshot dialog
    if let Some(mut dialog) = app.screenshot_dialog.take() {
        let keep_open = dialog.show(ctx, app);
        if keep_open {
            app.screenshot_dialog = Some(dialog);
        }
    }
```

- [ ] **Step 2: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -10
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/main_window.rs
git commit -m "feat: render ScreenshotDialog in main window"
```

---

### Task 8: Final Build and Test

- [ ] **Step 1: Full release build**

```bash
cargo build --release 2>&1 | tail -20
```

Expected: Build successful, warnings only about unused code

- [ ] **Step 2: Test frame 300 thumbnail generation**

1. Add a folder with videos to the app
2. Scan the folder
3. Check `%LOCALAPPDATA%/video-thumb-viewer/thumbs` directory
4. Thumbnails should be at frame 300 (or frame 1 if video is short)

- [ ] **Step 3: Test screenshot dialog**

1. Right-click on a video
2. Click "截圖..."
3. Dialog opens with:
   - Preview area
   - Timeline slider
   - Time/frame display
4. Drag slider - preview updates
5. Click "截圖" - saves custom thumbnail
6. Yellow dot indicator should appear on thumbnail

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "verify: thumbnail features working correctly"
```

---

## Spec Coverage Check

| Requirement | Task | Status |
|-------------|------|--------|
| Frame 300 thumbnail generation | Task 1, 2 | ✓ |
| Fallback to frame 1 | Task 1 | ✓ |
| Log warning on fallback | Task 1 | ✓ |
| Screenshot dialog with preview | Task 3 | ✓ |
| Timeline slider (no auto-play) | Task 3 | ✓ |
| 320x180 preview size | Task 3 | ✓ |
| Time/frame display | Task 3 | ✓ |
| Replace context menu input | Task 6 | ✓ |
| Dialog integration | Task 4, 5, 7 | ✓ |

## Placeholder Scan

- [x] No placeholders (TBD, TODO, etc.)
- [x] Actual code provided for all steps
- [x] Exact file paths specified
- [x] Commands with expected output

## Type Consistency Check

- `ThumbnailGenerator::generate_thumbnail_at_frame` signature matches usage ✓
- `ScreenshotDialog` fields consistent across struct/impl ✓
- `app.screenshot_dialog: Option<ScreenshotDialog>` in App ✓
- `file: FileEntry` parameter type consistent ✓

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-26-thumbnail-features.md`.**

Two execution options:

**1. Subagent-Driven (recommended)**
- I dispatch a fresh subagent per task, review between tasks, fast iteration
- Better for verifying each feature independently

**2. Inline Execution**
- Execute tasks in this session using executing-plans skill
- Batch execution with checkpoints for review

**Which approach would you prefer?**
