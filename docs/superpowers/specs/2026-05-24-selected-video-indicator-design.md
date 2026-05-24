# Selected Video Indicator Design

## Summary
Add single-click selection with a highlighted border and keyboard navigation (arrow keys + Enter) to the thumbnail grid.

## Requirements
- Single left-click on a thumbnail selects it
- Selected thumbnail shows a highlighted border
- Arrow keys (Left/Right) move selection; Up/Down moves by row; Enter plays selected video
- Selection clears when switching tabs or when the selected file is filtered out

## Architecture

### State
Add `selected_file_id: Option<i64>` to `App` struct (in `src/app.rs`).

### Selection on click
In `show_thumbnail_item` (`src/ui/main_window.rs`), add a `clicked()` check alongside the existing `double_clicked()` and `secondary_clicked()` checks. On single click, set `app.selected_file_id = Some(file.id)`.

### Visual indicator
In `show_thumbnail_item`, after drawing the thumbnail background, check if `app.selected_file_id == Some(file.id)`. If selected, draw a 2px colored border (using `egui::Style::visuals.selection.stroke` color for theme-aware highlighting) around the thumbnail cell.

### Keyboard navigation
In `App::update_impl`, after `MainWindow::show_main`, check for key presses:
- Left: move selection to previous file in `get_current_files()` list
- Right: move selection to next file
- Up: move by `items_per_row` positions back
- Down: move by `items_per_row` positions forward
- Enter: play the selected video

Items per row is estimated as `floor(available_width / (thumbnail_size + spacing))`.

On tab switch, clear `selected_file_id` to `None`.

## Files Changed
1. `src/app.rs` — add `selected_file_id: Option<i64>` field, initialize to `None`, clear on tab switch
2. `src/ui/main_window.rs` — add `clicked()` handler, draw selection border, add keyboard navigation

## Border Color
Use `ui.visuals().selection.stroke.color` for theme-aware highlighting, with 2px stroke width.
