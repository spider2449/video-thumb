# GUI Wiring Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix GUI wiring issues causing context menu not working, dialogs losing state, and thumbnail rendering layout problems.

**Architecture:** Store dialog instances in App struct to preserve state across frames, wire context menu to show on right-click, fix thumbnail grid layout using egui::Grid instead of manual row counting.

**Tech Stack:** Rust, egui (0.31), eframe (0.31)

---

## File Structure

| File | Responsibility | Changes |
|------|----------------|---------|
| `src/app.rs` | App state container - add dialog instances | Add `settings_dialog`, `tag_manager`, `context_menu` fields |
| `src/ui/main_window.rs` | Main UI rendering - wire dialogs properly | Use stored dialog instances, implement context menu trigger, fix grid layout |
| `src/ui/mod.rs` | Module exports - ensure types available | Already has exports, verify ContextMenu is exported |

---

## Background: Root Cause Analysis

The GUI issues were identified through systematic debugging:

1. **Context menu not working** (`main_window.rs:329`): `let _ = response.secondary_clicked()` - discards the click
2. **Dialogs lose state** (`main_window.rs:250,259`): `new()` called every frame creates fresh instances
3. **AddPathDialog unused**: Dialog logic duplicated inline in main_window.rs instead of using the struct
4. **Thumbnail layout broken**: Manual row/col counting with `horizontal_wrapped` doesn't work correctly

---

### Task 1: Add Dialog Instances to App State

**Files:**
- Modify: `src/app.rs:1-30` (add imports)
- Modify: `src/app.rs:71-93` (App struct fields)
- Modify: `src/app.rs:102-130` (App::new initialization)

**Prerequisites:** Read current src/app.rs to understand existing App struct

- [ ] **Step 1: Add dialog imports to app.rs**

Add to the existing imports in `src/app.rs`:

```rust
use crate::ui::{SettingsDialog, TagManager, ContextMenu};
```

- [ ] **Step 2: Add dialog instances to App struct**

In `src/app.rs`, add these fields to the `App` struct after `add_path_error`:

```rust
    // Dialog instances (stored to preserve state across frames)
    pub settings_dialog: Option<SettingsDialog>,
    pub tag_manager: Option<TagManager>,
    pub context_menu: ContextMenu,
```

- [ ] **Step 3: Initialize dialog instances in App::new**

In `App::new()` after the struct initialization (around line 90), add:

```rust
        app.settings_dialog = None;
        app.tag_manager = None;
        app.context_menu = ContextMenu::new();
```

- [ ] **Step 4: Build and verify no compilation errors**

```bash
cargo check 2>&1 | grep -E "^error" || echo "Build successful"
```

Expected: Only pre-existing warnings, no new errors related to these fields

- [ ] **Step 5: Commit the changes**

```bash
git add src/app.rs
git commit -m "chore: add dialog instances to App state for frame persistence"
```

---

### Task 2: Fix SettingsDialog State Persistence

**Files:**
- Modify: `src/ui/main_window.rs:248-266` (Settings dialog section)

- [ ] **Step 1: Replace inline SettingsDialog creation with stored instance**

Replace lines 249-256 in `main_window.rs`:

**OLD CODE:**
```rust
    // Settings Dialog
    if app.show_settings {
        let mut dialog = crate::ui::SettingsDialog::new();
        dialog.load(app);
        if !dialog.show(ctx, app) {
            app.show_settings = false;
        }
    }
```

**NEW CODE:**
```rust
    // Settings Dialog
    if app.show_settings {
        // Create dialog instance if doesn't exist
        if app.settings_dialog.is_none() {
            let mut dialog = crate::ui::SettingsDialog::new();
            dialog.load(app);
            app.settings_dialog = Some(dialog);
        }

        // Show the stored dialog
        if let Some(ref mut dialog) = app.settings_dialog {
            if !dialog.show(ctx, app) {
                app.show_settings = false;
                app.settings_dialog = None; // Clear when closed
            }
        }
    }
```

- [ ] **Step 2: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -20
```

Expected: No new errors

- [ ] **Step 3: Commit**

```bash
git add src/ui/main_window.rs
git commit -m "fix: settings dialog now persists state across frames"
```

---

### Task 3: Fix TagManager State Persistence

**Files:**
- Modify: `src/ui/main_window.rs:257-266` (Tag manager section)

- [ ] **Step 1: Replace inline TagManager creation with stored instance**

Replace lines 258-265 in `main_window.rs`:

**OLD CODE:**
```rust
    // Tag Manager Dialog
    if app.show_tag_manager {
        let mut dialog = crate::ui::TagManager::new();
        if dialog.show(ctx, app) {
            // Keep open
        } else {
            app.show_tag_manager = false;
        }
    }
```

**NEW CODE:**
```rust
    // Tag Manager Dialog
    if app.show_tag_manager {
        // Create dialog instance if doesn't exist
        if app.tag_manager.is_none() {
            app.tag_manager = Some(crate::ui::TagManager::new());
        }

        // Show the stored dialog
        if let Some(ref mut dialog) = app.tag_manager {
            if !dialog.show(ctx, app) {
                app.show_tag_manager = false;
                app.tag_manager = None; // Clear when closed
            }
        }
    }
```

- [ ] **Step 2: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -20
```

Expected: No new errors

- [ ] **Step 3: Commit**

```bash
git add src/ui/main_window.rs
git commit -m "fix: tag manager dialog now persists state across frames"
```

---

### Task 4: Fix Context Menu (Right-Click)

**Files:**
- Modify: `src/ui/main_window.rs:268-330` (show_thumbnail_item function)
- Modify: `src/ui/context_menu.rs:23-134` (ContextMenu::show signature)

**Prerequisites:** Read the ContextMenu::show method signature

- [ ] **Step 1: Check ContextMenu::show signature**

The context menu is designed to be used in two ways:
1. `ContextMenu::show()` displays the menu if there's a file set
2. The main_window should check right-click and set the file first

Current implementation in context_menu.rs creates its own `egui::Area` - this is correct for a floating menu.

- [ ] **Step 2: Modify show_thumbnail_item to trigger context menu**

Replace line 329 in `main_window.rs`:

**OLD CODE:**
```rust
    let _ = response.response.secondary_clicked();
```

**NEW CODE:**
```rust
    if response.response.secondary_clicked() {
        app.context_menu.file = Some(file.clone());
        app.context_menu.click_pos = response.response.interact_pointer_pos()
            .unwrap_or_else(|| response.response.rect.min);
    }
```

- [ ] **Step 3: Add context menu rendering after CentralPanel**

After the CentralPanel block ends (after line 180, before the Settings Dialog section), add:

```rust
    // Context menu overlay (rendered on top)
    app.context_menu.show(ctx, app);
```

Insert this after line 180 (`});` closing the CentralPanel) and before line 183 (`// Status bar`).

- [ ] **Step 4: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -20
```

Expected: No new errors

- [ ] **Step 5: Commit**

```bash
git add src/ui/main_window.rs
git commit -m "fix: context menu now shows on right-click"
```

---

### Task 5: Fix Thumbnail Grid Layout

**Files:**
- Modify: `src/ui/main_window.rs:156-180` (Main content section)

The current layout uses `horizontal_wrapped` incorrectly. We need to use `ui.horizontal_wrapped` properly or switch to `egui::Grid`.

- [ ] **Step 1: Replace broken layout with proper horizontal_wrapped**

Replace lines 156-180 in `main_window.rs`:

**OLD CODE:**
```rust
    // Main content
    egui::CentralPanel::default()
        .show(ctx, |ui| {
            let files = app.get_current_files();

            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    let mut col_count = 0;
                    for file in files {
                        if col_count >= cols {
                            ui.end_row();
                            col_count = 0;
                        }

                        if col_count == 0 {
                            ui.horizontal_wrapped(|ui| {
                                Self::show_thumbnail_item(ui, &file, app.thumbnail_size, ctx, app);
                            });
                        } else {
                            Self::show_thumbnail_item(ui, &file, app.thumbnail_size, ctx, app);
                        }
                        col_count += 1;
                    }
                });
        });
```

**NEW CODE:**
```rust
    // Main content
    egui::CentralPanel::default()
        .show(ctx, |ui| {
            let files = app.get_current_files();

            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for file in files {
                            Self::show_thumbnail_item(ui, &file, app.thumbnail_size, ctx, app);
                        }
                    });
                });
        });
```

- [ ] **Step 2: Build and verify**

```bash
cargo check 2>&1 | grep -E "^error" | head -20
```

Expected: No new errors

- [ ] **Step 3: Commit**

```bash
git add src/ui/main_window.rs
git commit -m "fix: thumbnail grid layout now renders all items correctly"
```

---

### Task 6: Verification - Run the Application

- [ ] **Step 1: Build release version**

```bash
cargo build --release 2>&1
```

Expected: Build completes successfully

- [ ] **Step 2: Run the application**

```bash
./target/release/video-thumb-viewer.exe &
```

Or if running directly:
```bash
cargo run --release 2>&1 | head -50
```

- [ ] **Step 3: Test each fix**

Verify the following behaviors:

1. **Settings Dialog State Persistence:**
   - Click Settings button (⚙️ 設定)
   - Type something in a field
   - Move mouse outside, click elsewhere (keep dialog open)
   - Field content should persist (not reset)

2. **Tag Manager State Persistence:**
   - Click Tag Manager button (🏷️ 標籤管理)
   - Type a tag name
   - Move mouse - text should stay

3. **Context Menu (Right-Click):**
   - Right-click on any thumbnail
   - Context menu should appear with options: Play, Rating stars, Tags, etc.
   - Press Escape - menu should close
   - Click outside - menu should close

4. **Thumbnail Grid Layout:**
   - If there are multiple videos, they should wrap to next row properly
   - Resize window - columns should adjust
   - All thumbnails should be visible, not just first row

- [ ] **Step 4: Check for warnings**

The build should show fewer dead code warnings for the UI components.

```bash
cargo build --release 2>&1 | grep -E "warning:.*(is never|are never)" | wc -l
```

Before: ~28 warnings  
After: Should be fewer (ContextMenu, SettingsDialog warnings should be gone)

- [ ] **Step 5: Final commit if all tests pass**

```bash
git add -A
git commit -m "verify: all GUI wiring fixes working correctly"
```

---

## Spec Coverage Check

| Requirement | Task | Status |
|-------------|------|--------|
| SettingsDialog persists state | Task 2 | ✓ |
| TagManager persists state | Task 3 | ✓ |
| ContextMenu shows on right-click | Task 4 | ✓ |
| Thumbnail grid renders all items | Task 5 | ✓ |

## Placeholder Scan

- [x] No "TBD" or "TODO"
- [x] No "implement later"
- [x] All code blocks contain actual code
- [x] Exact file paths and line numbers specified
- [x] Exact commands with expected output

## Type Consistency Check

- `SettingsDialog` stored as `Option<SettingsDialog>` ✓
- `TagManager` stored as `Option<TagManager>` ✓  
- `ContextMenu` stored directly (already used via app.context_menu) ✓
- `ContextMenu::show` signature matches usage ✓

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-26-gui-wiring-fix.md`.**

Two execution options:

**1. Subagent-Driven (recommended)**
- I dispatch a fresh subagent per task, review between tasks, fast iteration
- Better for verifying each fix independently

**2. Inline Execution**
- Execute tasks in this session using executing-plans skill
- Batch execution with checkpoints for review

**Which approach would you prefer?**
