use crate::app::App;
use crate::db::Tag;
use eframe::egui;

const PRESET_COLORS: &[&str] = &[
    "#FF5733", "#33FF57", "#3357FF", "#FF33F5", "#F5FF33",
    "#FF8C33", "#33FFF5", "#8C33FF", "#FF3333", "#33FF33",
    "#3333FF", "#FF33FF", "#FFFF33", "#33FFFF", "#808080",
];

pub struct TagManager {
    pub new_tag_name: String,
    pub new_tag_color: String,
    pub editing_tag_id: Option<i64>,
    pub edit_name: String,
    pub edit_color: String,
    pub error: Option<String>,
}

impl TagManager {
    pub fn new() -> Self {
        Self {
            new_tag_name: String::new(),
            new_tag_color: "#808080".to_string(),
            editing_tag_id: None,
            edit_name: String::new(),
            edit_color: String::new(),
            error: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) -> bool {
        let mut open = true;

        egui::Window::new("標籤管理")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(400.0);
                ui.set_min_height(300.0);

                ui.collapsing("現有標籤", |ui| {
                    let tags_snapshot: Vec<_> = app.tags.clone();
                    for tag in &tags_snapshot {
                        let color = crate::utils::hex_to_color(&tag.color);

                        ui.horizontal(|ui| {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(ui.cursor().min, [12.0, 12.0].into()),
                                0.0,
                                color,
                            );
                            ui.add_space(16.0);
                            ui.label(&tag.name);

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let tag_id = tag.id;
                                if ui.button("刪除").clicked() {
                                    app.db.blocking_lock().delete_tag(tag_id).ok();
                                    app.tags.retain(|t| t.id != tag_id);
                                }
                                if ui.button("編輯").clicked() {
                                    self.editing_tag_id = Some(tag_id);
                                    self.edit_name = tag.name.clone();
                                    self.edit_color = tag.color.clone();
                                }
                            });
                        });
                    }
                });

                ui.separator();

                if let Some(editing_id) = self.editing_tag_id {
                    ui.group(|ui| {
                        ui.label("編輯標籤:");
                        ui.text_edit_singleline(&mut self.edit_name);
                        ui.horizontal(|ui| {
                            ui.label("顏色:");
                            ui.text_edit_singleline(&mut self.edit_color);
                        });

                        self.show_color_presets(ui, true);

                        ui.horizontal(|ui| {
                            if ui.button("取消").clicked() {
                                self.editing_tag_id = None;
                            }
                            if ui.button("儲存").clicked() {
                                self.save_tag_edit(editing_id, app);
                            }
                        });
                    });
                } else {
                    ui.group(|ui| {
                        ui.label("新增標籤:");
                        ui.text_edit_singleline(&mut self.new_tag_name);

                        ui.horizontal(|ui| {
                            ui.label("顏色:");
                            ui.text_edit_singleline(&mut self.new_tag_color);
                        });

                        self.show_color_presets(ui, false);

                        if ui.button("新增").clicked() {
                            self.add_new_tag(app);
                        }

                        if let Some(error) = &self.error {
                            ui.colored_label(egui::Color32::RED, error);
                        }
                    });
                }
            });

        if !open {
            self.reset();
        }

        open
    }

    fn show_color_presets(&mut self, ui: &mut egui::Ui, is_edit: bool) {
        ui.horizontal_wrapped(|ui| {
            for color in PRESET_COLORS {
                let c = crate::utils::hex_to_color(color);
                let rect_min = ui.cursor().min;
                let rect = egui::Rect::from_min_size(rect_min, [20.0, 20.0].into());

                ui.painter().rect_filled(rect.shrink(1.0), 0.0, c);
                ui.add_space(24.0);

                if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if rect.contains(hover_pos) && ui.input(|i| i.pointer.any_click()) {
                        if is_edit {
                            self.edit_color = color.to_string();
                        } else {
                            self.new_tag_color = color.to_string();
                        }
                    }
                }
            }
        });
    }

    fn save_tag_edit(&mut self, tag_id: i64, app: &mut App) {
        let name = self.edit_name.trim();
        if name.is_empty() {
            self.error = Some("標籤名稱不能為空".to_string());
            return;
        }

        let db = app.db.blocking_lock();
        if db.update_tag(tag_id, name, &self.edit_color).is_ok() {
            if let Some(tag) = app.tags.iter_mut().find(|t| t.id == tag_id) {
                tag.name = name.to_string();
                tag.color = self.edit_color.clone();
            }
        }
        self.editing_tag_id = None;
    }

    fn add_new_tag(&mut self, app: &mut App) {
        let name = self.new_tag_name.trim();
        if name.is_empty() {
            self.error = Some("標籤名稱不能為空".to_string());
            return;
        }

        let db = app.db.blocking_lock();
        match db.create_tag(name, &self.new_tag_color) {
            Ok(id) => {
                app.tags.push(Tag {
                    id,
                    name: name.to_string(),
                    color: self.new_tag_color.clone(),
                });
                app.tags.sort_by(|a, b| a.name.cmp(&b.name));
                self.new_tag_name.clear();
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("建立標籤失敗: {}", e));
            }
        }
    }

    pub fn reset(&mut self) {
        self.new_tag_name.clear();
        self.new_tag_color = "#808080".to_string();
        self.editing_tag_id = None;
        self.edit_name.clear();
        self.edit_color.clear();
        self.error = None;
    }
}

impl Default for TagManager {
    fn default() -> Self {
        Self::new()
    }
}