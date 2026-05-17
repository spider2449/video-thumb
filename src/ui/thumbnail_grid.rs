use crate::app::App;
use crate::db::FileEntry;
use crate::thumbnail::ThumbnailGenerator;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct ThumbnailGrid {
    pub context_menu: crate::ui::ContextMenu,
}

thread_local! {
    static TEXTURE_CACHE: Mutex<HashMap<String, egui::TextureHandle>> = Mutex::new(HashMap::new());
}

impl ThumbnailGrid {
    pub fn new() -> Self {
        Self {
            context_menu: crate::ui::ContextMenu::new(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) {
        let files = app.get_current_files();
        let thumb_size = app.thumbnail_size;
        let available_width = ctx.input(|i| i.screen_rect.width()) - 40.0;
        let cols = ((available_width / thumb_size) as usize).max(1);

        egui::CentralPanel::default()
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        let mut row_count = 0;
                        let mut col_count = 0;

                        for file in files.iter() {
                            if col_count >= cols {
                                ui.end_row();
                                col_count = 0;
                                row_count += 1;
                            }

                            if col_count == 0 {
                                ui.horizontal_wrapped(|ui| {
                                    self.show_single_thumbnail(ui, file, thumb_size, app, ctx);
                                });
                            } else {
                                self.show_single_thumbnail(ui, file, thumb_size, app, ctx);
                            }

                            col_count += 1;
                        }
                    });

                if self.context_menu.file.is_some() {
                    self.context_menu.show(ctx, app);
                }
            });
    }

    fn show_single_thumbnail(&mut self, ui: &mut egui::Ui, file: &FileEntry, size: f32, app: &App, ctx: &egui::Context) {
        let response = ui.allocate_ui([size, size + 50.0].into(), |ui| {
            ui.vertical(|ui| {
                let thumb_size_inner = size - 4.0;
                let thumb_rect = ui.allocate_rect(
                    egui::Rect::from_min_size(ui.cursor().min, [thumb_size_inner, thumb_size_inner].into()),
                    egui::Sense::click_and_drag(),
                );

                ui.painter().rect_filled(
                    thumb_rect.rect.shrink(1.0),
                    0.0,
                    egui::Color32::from_gray(30),
                );

                if let Some(thumb_path) = &file.thumbnail_path {
                    let path = PathBuf::from(thumb_path);
                    if path.exists() {
                        if let Some(tex_handle) = load_texture(ctx, &path) {
                            let tex_id = tex_handle.id();
                            let img_rect = egui::Rect::from_min_size(thumb_rect.rect.min, thumb_rect.rect.size());
                            ui.painter().image(tex_id, img_rect, egui::Rect::from_min_max([0.0, 0.0].into(), [1.0, 1.0].into()), egui::Color32::WHITE);
                        }
                    } else {
                        ui.painter().rect_filled(
                            thumb_rect.rect.shrink(1.0),
                            0.0,
                            egui::Color32::from_gray(50),
                        );
                        ui.painter().text(
                            thumb_rect.rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "?",
                            egui::FontId::proportional(24.0),
                            egui::Color32::WHITE,
                        );
                    }
                } else {
                    ui.painter().rect_filled(
                        thumb_rect.rect.shrink(1.0),
                        0.0,
                        egui::Color32::from_gray(50),
                    );
                    ui.painter().text(
                        thumb_rect.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "?",
                        egui::FontId::proportional(24.0),
                        egui::Color32::DARK_GRAY,
                    );
                }

                if file.has_custom_thumb {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(thumb_rect.rect.min, [8.0, 8.0].into()),
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(255, 200, 0, 200),
                    );
                }

                ui.label(
                    egui::RichText::new(truncate_filename(&file.file_name, 20))
                        .size(12.0)
                        .color(egui::Color32::WHITE),
                );

                ui.horizontal(|ui| {
                    for star in 1..=5 {
                        let color = if star <= file.rating {
                            egui::Color32::from_rgb(255, 200, 0)
                        } else {
                            egui::Color32::from_gray(80)
                        };
                        let text = egui::RichText::new(if star <= file.rating { "★" } else { "☆" })
                            .color(color)
                            .size(14.0);
                        ui.label(text);
                    }
                });
            });
        });

        let root_dir = get_data_dir();

        if response.response.double_clicked() {
            let file_path = file.file_path.clone();
            let db = app.db.blocking_lock();
            let player = db.get_setting("default_player")
                .ok()
                .flatten()
                .unwrap_or_default();
            drop(db);
            if let Some(ffmpeg_path) = get_ffmpeg_path() {
                let generator = ThumbnailGenerator::new(&root_dir, &ffmpeg_path);
                generator.play_video(&file_path, &player).ok();
            }
        }

        if response.response.secondary_clicked() {
            let file_copy = file.clone();
            self.context_menu.file = Some(file_copy);
            self.context_menu.click_pos = response.response.rect.min;
            self.context_menu.time_input.clear();
        }
    }
}

impl Default for ThumbnailGrid {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_filename(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        let ext = std::path::Path::new(name)
            .extension()
            .map(|e| e.len())
            .unwrap_or(0);
        let name_len = max_len.saturating_sub(ext + 4);
        if name_len > 0 && ext > 0 {
            format!("{}...{}", &name[..name_len], &name[name.len() - ext..])
        } else {
            name[..max_len.min(name.len())].to_string()
        }
    }
}

fn load_texture(ctx: &egui::Context, path: &PathBuf) -> Option<egui::TextureHandle> {
    let path_str = path.to_string_lossy().to_string();

    TEXTURE_CACHE.with(|cache| {
        let mut cache = cache.lock().ok()?;

        if let Some(handle) = cache.get(&path_str) {
            return Some(handle.clone());
        }

        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];

        let colors: Vec<egui::Color32> = rgba.pixels()
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();
        let color_img = egui::ColorImage { size, pixels: colors };

        let handle = ctx.load_texture(&path_str, color_img, egui::TextureOptions::LINEAR);

        cache.insert(path_str, handle.clone());
        Some(handle)
    })
}

fn get_ffmpeg_path() -> Option<PathBuf> {
    let root_dir = get_data_dir();
    let ffmpeg_dir = root_dir.join("ffmpeg");

    #[cfg(target_os = "windows")]
    {
        let path = ffmpeg_dir.join("ffmpeg.exe");
        if path.exists() {
            return Some(path);
        }
        if std::process::Command::new("where")
            .arg("ffmpeg")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(PathBuf::from("ffmpeg"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let path = ffmpeg_dir.join("ffmpeg");
        if path.exists() {
            return Some(path);
        }
        return Some(PathBuf::from("ffmpeg"));
    }

    None
}

fn get_data_dir() -> std::path::PathBuf {
    crate::utils::get_data_dir()
}
