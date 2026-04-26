use crate::app::App;
use eframe::egui;

pub struct TabBar {}

impl TabBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) {
        egui::TopBottomPanel::top("tab_bar")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    egui::ScrollArea::horizontal()
                        .drag_to_scroll(false)
                        .show(ui, |ui| {
                            let paths_snapshot: Vec<_> = app.watched_paths.clone();
                            let mut close_request: Option<i64> = None;
                            for path in &paths_snapshot {
                                let selected = app.current_path_id == Some(path.id);
                                let button = egui::Button::new(&path.display_name)
                                    .selected(selected);

                                if ui.add(button).clicked() {
                                    app.current_path_id = Some(path.id);
                                    if !app.files.contains_key(&path.id) {
                                        app.load_files_for_path(path.id);
                                    }
                                }

                                if selected {
                                    let close_button = ui.button("×");
                                    if close_button.clicked() {
                                        close_request = app.current_path_id;
                                    }
                                }
                            }

                            if let Some(id) = close_request {
                                let db = app.db.blocking_lock();
                                let should_remove = db.remove_path(id).is_ok();
                                drop(db);
                                if should_remove {
                                    app.watched_paths.retain(|p| p.id != id);
                                    app.files.remove(&id);
                                    app.current_path_id = app.watched_paths.first().map(|p| p.id);
                                    if let Some(new_id) = app.current_path_id {
                                        app.load_files_for_path(new_id);
                                    }
                                }
                            }
                        });

                    ui.separator();

                    if ui.button("+ 新增路徑").clicked() {
                        app.show_add_path = true;
                    }
                });
            });
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}