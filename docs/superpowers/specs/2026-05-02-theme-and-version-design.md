# Theme Switching and Version Display Design

**Date:** 2026-05-02
**Status:** Approved

## Overview

Add switchable light/dark theme support and display application version in the title bar.

## Features

### 1. Toolbar Theme Toggle

**Behavior:**
- Add a toggle button to the toolbar with sun/moon icons
- Clicking toggles between light and dark themes using egui's built-in `Visuals`
- Theme preference persists in database via `set_setting("theme", "light"|"dark")`
- Load saved preference on app startup (default to dark if not set)

**Icons:**
- Sun icon (☀) for light mode
- Moon icon (☽) for dark mode

### 2. Version in Title Bar

**Behavior:**
- Update window title from "Video Thumb Viewer" to "Video Thumb Viewer v0.1.0"
- Maintain `VERSION` constant that's synced with `Cargo.toml` version

## Files to Modify

| File | Changes |
|------|---------|
| `src/main.rs` | Update window title to include version |
| `src/app.rs` | Add `current_theme` field, toolbar theme button, load/save theme |
| `src/ui/settings_dialog.rs` | Optionally show current theme (toolbar is primary UI) |

## Implementation Notes

- Use `egui::Visuals::light()` / `egui::Visuals::dark()` for built-in theme switching
- Theme toggle button in toolbar - no separate Settings UI needed
- Version constant in `app.rs` or `main.rs`, e.g., `pub const VERSION: &str = "0.1.0";`