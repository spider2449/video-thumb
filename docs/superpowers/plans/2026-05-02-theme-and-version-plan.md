# Theme Switching and Version Display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add switchable light/dark theme via toolbar toggle and display version in window title bar.

**Architecture:**
- Theme preference stored in database via `set_setting("theme", "light"|"dark")`
- Theme applied using egui's built-in `Visuals::light()` / `Visuals::dark()` methods
- Version displayed by appending to window title in `main.rs`

**Tech Stack:** egui/eframe 0.31

---

## File Changes Summary

| File | Changes |
|------|---------|
| `src/main.rs` | Add `VERSION` constant; update `eframe::run_native` title to include version |
| `src/app.rs` | Add `current_theme: String` field; load theme on first frame |
| `src/ui/main_window.rs` | Add theme toggle button (☀/☽) to toolbar that toggles and saves theme |

---

## Tasks

### Task 1: Add VERSION constant and update window title in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add VERSION constant at top of main.rs**

Add this line after `use app::App;` (around line 35):

```rust
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 2: Update window title**

Find `eframe::run_native(` and change title string:

```rust
eframe::run_native(
    &format!("Video Thumb Viewer v{}", VERSION),
```

- [ ] **Step 3: Build and commit**

Run: `cargo build --quiet 2>&1 | head -20`

Commit:
```bash
git add src/main.rs && git commit -m "feat: add VERSION constant and display in window title"
```

---

### Task 2: Add theme storage to App struct

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add current_theme field to App struct**

In `src/app.rs` around line 40, add after `console_visible`:

```rust
pub current_theme: String,
```

- [ ] **Step 2: Initialize current_theme in App::new()**

In `src/app.rs` around line 105, add to struct literal:

```rust
current_theme: "dark".to_string(),
```

- [ ] **Step 3: Add apply_theme method**

Add this method in `src/app.rs` after `reset_add_path_dialog` (around line 233):

```rust
pub fn apply_theme(&self, ctx: &egui::Context) {
    if self.current_theme == "light" {
        ctx.set_visuals(egui::Visuals::light());
    } else {
        ctx.set_visuals(egui::Visuals::dark());
    }
}
```

- [ ] **Step 4: Load theme and apply in update_impl on first frame**

Since `App::new()` lacks `egui::Context`, we load theme in `update_impl`. Add this at the START of `update_impl` (around line 235), BEFORE the F12 toggle:

```rust
// Load and apply theme on first frame
if self.current_theme == "dark" {
    let db_guard = self.db.blocking_lock();
    self.current_theme = db_guard.get_setting("theme")
        .ok()
        .flatten()
        .unwrap_or_else(|| "dark".to_string());
    drop(db_guard);
    self.apply_theme(ctx);
}
```

- [ ] **Step 5: Build and commit**

Run: `cargo build --quiet 2>&1 | head -20`

Commit:
```bash
git add src/app.rs && git commit -m "feat: add theme storage and apply_theme method"
```

---

### Task 3: Add theme toggle button to toolbar

**Files:**
- Modify: `src/ui/main_window.rs`

- [ ] **Step 1: Add theme toggle button to toolbar**

Find `ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {` (around line 98) and add theme toggle button BEFORE the settings button:

Replace the ending section:
```rust
if ui.button("⚙️ 設定").clicked() {
    app.show_settings = true;
}
```

With:
```rust
    // Theme toggle button
    let theme_icon = if app.current_theme == "light" { "☽" } else { "☀" };
    if ui.button(theme_icon).clicked() {
        app.current_theme = if app.current_theme == "light" {
            "dark".to_string()
        } else {
            "light".to_string()
        };
        // Apply theme immediately
        if app.current_theme == "light" {
            ctx.set_visuals(egui::Visuals::light());
        } else {
            ctx.set_visuals(egui::Visuals::dark());
        }
        // Save preference
        let db = app.db.blocking_lock();
        let _ = db.set_setting("theme", &app.current_theme);
    }

    if ui.button("⚙️ 設定").clicked() {
        app.show_settings = true;
    }
```

- [ ] **Step 2: Build to verify**

Run: `cargo build --quiet 2>&1 | head -30`

- [ ] **Step 3: Run and test**

Run: `cargo run`
Verify:
- Window title shows "Video Thumb Viewer v0.1.0"
- Theme toggle button (☀) visible in toolbar
- Clicking button switches theme
- Theme persists after restart

- [ ] **Step 4: Commit**

```bash
git add src/ui/main_window.rs && git commit -m "feat(ui): add theme toggle button to toolbar"
```

---

## Verification Checklist

After all tasks complete:
- [ ] Window title shows "Video Thumb Viewer v{VERSION}"
- [ ] Toolbar shows ☀/☽ theme toggle button
- [ ] Clicking toggle switches between light and dark themes
- [ ] Theme preference persists after app restart
- [ ] Default theme is dark