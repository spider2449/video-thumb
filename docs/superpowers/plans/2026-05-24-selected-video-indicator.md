# Selected Video Indicator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add single-click selection, highlighted border, and keyboard navigation to the thumbnail grid.

**Architecture:** Add `selected_file_id: Option<i64>` to `App` struct. In `show_thumbnail_item`, handle `clicked()` to set selection and draw a 2px border around the selected thumbnail. In `update_impl`, handle arrow keys to navigate selection and Enter to play.

**Tech Stack:** Rust, egui/eframe v0.31

---

### Task 1: Add selected_file_id state to App

**Files:**
- Modify: `src/app.rs:8-48` (struct fields), `src/app.rs:82-117` (constructor)

- [ ] **Step 1: Add field to App struct**

In `src/app.rs`, add `selected_file_id: Option<i64>` after `pub current_path_id: Option<i64>,` (line 14).

```rust
    pub current_path_id: Option<i64>,
    pub selected_file_id: Option<i64>,
```

- [ ] **Step 2: Initialize in constructor**

In `src/app.rs` line 88 area, add `selected_file_id: None,` before `add_path_path: String::new(),` (around line 103).

```rust
            current_path_id: None,
            selected_file_id: None,
            thumbnail_size: 200.0,
```

- [ ] **Step 3: Clear selection on tab switch**

In `src/ui/main_window.rs`, in the tab bar click handler (around line 151-156), add `app.selected_file_id = None;` after setting `current_path_id`:

```rust
                                if ui.add(button).clicked() {
                                    app.current_path_id = Some(path.id);
                                    app.selected_file_id = None;
                                    if !app.files.contains_key(&path.id) {
                                        app.load_files_for_path(path.id);
                                    }
                                }
```

---

### Task 2: Add single-click selection and highlight border

**Files:**
- Modify: `src/ui/main_window.rs:353-462` (show_thumbnail_item)

- [ ] **Step 1: Add click handling and selection border in show_thumbnail_item**

In `src/ui/main_window.rs`, replace the existing `show_thumbnail_item` function body. The changes are:

1. After drawing the background rect (line 365-369), add a selection border
2. Replace the click detection logic at lines 432-461 to handle single click

The modified function (key changes only — replace lines 365-461):

First, add selection border drawing right after the background fill (after line 369):

```rust
                // selection highlight border
                let is_selected = app.selected_file_id == Some(file.id);
                if is_selected {
                    ui.painter().rect_stroke(
                        thumb_rect.rect.shrink(1.0),
                        0.0,
                        egui::Stroke::new(3.0, ui.visuals().selection.stroke.color),
                    );
                }
```

Then replace the click handling section (lines 432-461) with:

```rust
        // Check thumb_rect for clicks
    let thumb_response = match &*thumb_rc.borrow() { Some(r) => r.clone(), None => return };

    if thumb_response.double_clicked() {
        log::info!("Double-click detected on: {}", file.file_name);
            let file_path = file.file_path.clone();
            let player = app.db.blocking_lock().get_setting("default_player")
                .ok().flatten().unwrap_or_default();
            let player_str: &str = if player.is_empty() { "" } else { &player };
            if let Some(ffp) = crate::utils::get_ffmpeg_path() {
                let gen = crate::thumbnail::ThumbnailGenerator::new(&crate::utils::get_data_dir(), &ffp);
                if let Err(e) = gen.play_video(&file_path, player_str) {
            log::error!("Failed to play video: {}", e);
        }
            }
        }

        if thumb_response.clicked() && !thumb_response.double_clicked() {
            app.selected_file_id = Some(file.id);
        }

        if thumb_response.secondary_clicked() {
        if app.context_menu.is_none() {
            let mut menu = crate::ui::ContextMenu::new();
            menu.file = Some(file.clone());
            menu.click_pos = thumb_response.interact_pointer_pos()
                .unwrap_or_else(|| response.response.rect.min);
            app.context_menu = Some(menu);
        } else {
            if let Some(ref mut menu) = app.context_menu {
                menu.file = Some(file.clone());
                menu.click_pos = response.response.interact_pointer_pos()
                    .unwrap_or_else(|| response.response.rect.min);
            }
        }
    }
```

Note: The `clicked() && !double_clicked()` guard prevents selection from also triggering on double-click.

---

### Task 3: Add keyboard navigation (arrow keys + Enter)

**Files:**
- Modify: `src/app.rs:418-425` (update method), `src/ui/main_window.rs:14-16` (may need available_width)

- [ ] **Step 1: Add keyboard handling to update_impl**

In `src/app.rs`, at the end of `update_impl` (after line 418 `MainWindow::show_main(ctx, self);`), add keyboard navigation logic:

```rust
        // Keyboard navigation for selected video
        if let Some(selected_id) = self.selected_file_id {
            let current_files = self.get_current_files();
            if let Some(selected_idx) = current_files.iter().position(|f| f.id == selected_id) {
                let screen_width = ctx.input(|i| i.screen_rect.width());
                let spacing = 10.0;
                let items_per_row = ((screen_width - 40.0) / (self.thumbnail_size + spacing)).max(1.0) as usize;

                let new_idx = if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                    Some(selected_idx.saturating_add(1).min(current_files.len().saturating_sub(1)))
                } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                    Some(selected_idx.saturating_sub(1))
                } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    Some((selected_idx + items_per_row).min(current_files.len().saturating_sub(1)))
                } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    Some(selected_idx.saturating_sub(items_per_row))
                } else if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Play the selected video
                    if let Some(file) = current_files.get(selected_idx) {
                        let file_path = file.file_path.clone();
                        let db = self.db.blocking_lock();
                        let player = db.get_setting("default_player")
                            .ok().flatten().unwrap_or_default();
                        drop(db);
                        if let Some(ffp) = crate::utils::get_ffmpeg_path() {
                            let gen = crate::thumbnail::ThumbnailGenerator::new(&crate::utils::get_data_dir(), &ffp);
                            let _ = gen.play_video(&file_path, if player.is_empty() { "" } else { &player });
                        }
                    }
                    None
                } else {
                    None
                };

                if let Some(idx) = new_idx {
                    self.selected_file_id = Some(current_files[idx].id);
                    // Scroll into view hint
                    ctx.request_repaint();
                }
            } else {
                // Selected file no longer in the current list (filtered out)
                self.selected_file_id = None;
            }
        }
```

- [ ] **Step 2: Ensure Enter on unfocused grid doesn't conflict with other Enter handlers**

No changes needed — the Enter key check only applies when a video is selected (`selected_file_id.is_some()`).

---

### Task 4: Build and verify

- [ ] **Step 1: Build the project**

```bash
cargo build 2>&1
```
Expected: Compilation succeeds with no errors.

- [ ] **Step 2: Run the project**

```bash
cargo run 2>&1
```
Expected: Application launches. Clicking a thumbnail selects it with a blue highlight border. Arrow keys navigate. Enter plays the selected video.
