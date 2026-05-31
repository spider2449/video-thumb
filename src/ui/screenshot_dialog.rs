use crate::app::App;
use crate::db::FileEntry;
use eframe::egui;
use std::sync::Arc;
use std::time::Instant;

const DEBOUNCE_MS: u128 = 100;

pub struct ScreenshotDialog {
    pub file: FileEntry,
    pub current_time: f64,
    pub total_duration: f64,
    pub preview_texture: Option<egui::TextureHandle>,
    pub is_loading: bool,
    pub thumbnail_size: [u32; 2],
    pub last_update_time: Instant,
    pub last_preview_time: f64,  // Time shown in current preview
}

impl ScreenshotDialog {
    pub fn new(file: FileEntry) -> Self {
        // Get video duration using ffmpeg
        let total_duration = Self::get_video_duration(&file.file_path).unwrap_or(60.0);

        // Start at 5 seconds (or 1 second if video is short)
        let start_time = if total_duration > 5.0 { 5.0 } else { 1.0 };

        Self {
            file,
            current_time: start_time,
            total_duration,
            preview_texture: None,
            is_loading: false,
            thumbnail_size: [320, 180],
            last_update_time: Instant::now(),
            last_preview_time: -1.0,  // No preview yet
        }
    }

    fn get_video_duration(video_path: &str) -> Option<f64> {
        // Use ffprobe to get duration - try to find ffprobe next to ffmpeg
        let ffprobe_path = crate::utils::get_ffprobe_path()?;

        let output = std::process::Command::new(&ffprobe_path)
            .args(["-v", "error"])
            .args(["-show_entries", "format=duration"])
            .args(["-of", "default=noprint_wrappers=1:nokey=1"])
            .arg(video_path)
            .output()
            .ok()?;

        let duration_str = String::from_utf8_lossy(&output.stdout);
        duration_str.trim().parse::<f64>().ok()
    }

    /// Returns true to keep dialog open
    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) -> bool {
        let close_requested = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let capture_requested = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let close_clone = close_requested.clone();
        let capture_clone = capture_requested.clone();
        let lang = app.lang;

        egui::Window::new(crate::lang::t(lang, "screenshot_title"))
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_size([600.0, 500.0])
            .min_size([500.0, 450.0])
            .show(ctx, |ui| {
                let _available_width = ui.available_width();

                ui.group(|ui| {
                    ui.set_min_height(200.0);

                    if let Some(texture) = &self.preview_texture {
                        let size = egui::Vec2::new(
                            self.thumbnail_size[0] as f32,
                            self.thumbnail_size[1] as f32,
                        );
                        let available_rect = ui.available_rect_before_wrap();
                        let img_pos = available_rect.center() - (size / 2.0);
                        ui.put(
                            egui::Rect::from_min_size(img_pos, size),
                            egui::Image::new(texture).fit_to_exact_size(size),
                        );
                    } else if self.is_loading {
                        ui.centered_and_justified(|ui| {
                            ui.label(crate::lang::t(lang, "loading"));
                        });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label(crate::lang::t(lang, "drag_timeline"));
                        });
                    }
                });

                ui.add_space(8.0);

                let prev_time = self.current_time;
                ui.label(format!("{}{:?}", crate::lang::t(lang, "time_prefix"), self.current_time));
                ui.add(
                    egui::Slider::new(&mut self.current_time, 0.0..=self.total_duration)
                        .show_value(true)
                        .text(crate::lang::t(lang, "timeline")),
                );

                let total_mins = (self.total_duration / 60.0) as u32;
                let total_secs = (self.total_duration % 60.0) as u32;
                let mins = (self.current_time / 60.0) as u32;
                let secs = (self.current_time % 60.0) as u32;
                ui.horizontal(|ui| {
                    ui.label(format!("{}{:02}:{:02}", crate::lang::t(lang, "current_prefix"), mins, secs));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{}{:02}:{:02}", crate::lang::t(lang, "total_prefix"), total_mins, total_secs));
                    });
                });

        let time_changed_significantly = (self.current_time - prev_time).abs() > 0.001;
        if time_changed_significantly {
            self.last_update_time = Instant::now();
        }

        let elapsed = self.last_update_time.elapsed().as_millis();
        let preview_pending = (self.current_time - self.last_preview_time).abs() > 0.001;
        if elapsed >= DEBOUNCE_MS && !self.is_loading && preview_pending {
            self.update_preview(ctx);
            self.last_preview_time = self.current_time;
        }

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button(crate::lang::t(lang, "cancel")).clicked() {
                        close_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(crate::lang::t(lang, "capture")).clicked() {
                            capture_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                            close_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    });
                });
            });

        let capture = capture_requested.load(std::sync::atomic::Ordering::Relaxed);
        let close = close_requested.load(std::sync::atomic::Ordering::Relaxed);

        if capture {
            self.capture_screenshot(app);
        }

        !close
    }

    fn update_preview(&mut self, ctx: &egui::Context) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;

        let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() else {
            log::error!("ffmpeg not found");
            self.is_loading = false;
            return;
        };

        let video_path = self.file.file_path.clone();
        let time = self.current_time;
        let size = self.thumbnail_size;

        // Generate preview at current time
        let data_dir = crate::utils::get_data_dir();

        // Create temp preview file
        let temp_path = data_dir.join(format!("preview_{}.jpg", self.file.id));

        let time_str = format!("{:02}:{:02}:{:02}.{:03}",
            (time / 3600.0) as u32,
            ((time % 3600.0) / 60.0) as u32,
            (time % 60.0) as u32,
            ((time % 1.0) * 1000.0) as u32
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
                            format!("preview_{}", self.file.id),
                            color_img,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    // Clean up temp file
                    let _ = std::fs::remove_file(&temp_path);
                } else {
                    log::warn!("Failed to generate preview: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
            Err(e) => {
                log::error!("Failed to generate preview: {}", e);
            }
        }
        // Ensure temp file is always cleaned up
        let _ = std::fs::remove_file(&temp_path);

        self.is_loading = false;
    }

    fn capture_screenshot(&mut self, app: &mut App) {
        let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() else {
            log::error!("ffmpeg not found");
            return;
        };

        let data_dir = crate::utils::get_data_dir();

        let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path, app.db.clone());

        if let Ok(thumb_path) = gen.generate_thumbnail_at_time(&self.file.file_path, self.current_time, Some(self.file.id)) {
            let db_result = app.db.lock().unwrap()
                .update_thumbnail(self.file.id, &thumb_path);
            if let Err(e) = db_result {
                log::error!("Failed to update thumbnail in database: {}", e);
            } else {
                for files in app.files.values_mut() {
                    let mut vec = (**files).clone();
                    for f in vec.iter_mut() {
                        if f.id == self.file.id {
                            f.thumbnail_path = Some(thumb_path.clone());
                            f.has_custom_thumb = true;
                        }
                    }
                    *files = Arc::new(vec);
                }
                app.invalidate_cache();
                log::info!("Screenshot saved: {}", thumb_path);
            }
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
            last_update_time: Instant::now(),
            last_preview_time: -1.0,
        }
    }
}
