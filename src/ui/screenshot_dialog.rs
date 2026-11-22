use super::icons::Icon;
use crate::app::App;
use crate::db::{FileEntry, SafeMutexExt};
use eframe::egui;
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(100);

type PreviewResult = Result<image::RgbaImage, PreviewError>;

#[derive(Debug)]
enum PreviewError {
    MissingFfmpeg,
    MissingFile(String),
    NoFrame,
    Failed(String),
}

impl PreviewError {
    fn message(&self, lang: crate::lang::Lang) -> String {
        let key = match self {
            Self::MissingFfmpeg => "preview_ffmpeg_missing",
            Self::MissingFile(_) => "preview_file_missing",
            Self::NoFrame => "preview_no_frame",
            Self::Failed(_) => "preview_failed",
        };
        let message = crate::lang::t(lang, key);
        match self {
            Self::Failed(details) => match details.lines().find(|line| !line.trim().is_empty()) {
                Some(line) => format!("{message}\n{line}"),
                None => message.into(),
            },
            _ => message.into(),
        }
    }

    fn details(&self) -> Option<&str> {
        match self {
            Self::MissingFile(path) => Some(path),
            Self::Failed(details) => Some(details),
            _ => None,
        }
    }
}

pub struct ScreenshotDialog {
    pub file: FileEntry,
    pub current_time: f64,
    pub total_duration: f64,
    pub preview_texture: Option<egui::TextureHandle>,
    pub thumbnail_size: [u32; 2],
    pub last_update_time: Instant,
    pub last_preview_time: f64,
    preview_rx: Option<std::sync::mpsc::Receiver<PreviewResult>>,
    duration_rx: Option<std::sync::mpsc::Receiver<f64>>,
    preview_error: Option<PreviewError>,
}

impl ScreenshotDialog {
    pub fn new(file: FileEntry, ctx: &egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let video_path = file.file_path.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let duration = Self::get_video_duration(&video_path).unwrap_or(60.0);
            if tx.send(duration).is_ok() {
                ctx.request_repaint();
            }
        });

        Self {
            file,
            current_time: 0.0,
            total_duration: 60.0,
            preview_texture: None,
            thumbnail_size: [320, 180],
            last_update_time: Instant::now(),
            last_preview_time: -1.0,
            preview_rx: None,
            duration_rx: Some(rx),
            preview_error: None,
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

        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|duration| duration.is_finite() && *duration > 0.0)
    }

    /// Returns true to keep dialog open
    pub fn show(&mut self, ctx: &egui::Context, app: &mut App) -> bool {
        let Some((close, capture)) = self
            .show_window(ctx, app.lang)
            .and_then(|window| window.inner)
        else {
            return true;
        };

        if capture {
            self.capture_screenshot(app);
        }

        !close
    }

    fn show_window(
        &mut self,
        ctx: &egui::Context,
        lang: crate::lang::Lang,
    ) -> Option<egui::InnerResponse<Option<(bool, bool)>>> {
        let default_size = egui::vec2(520.0, 380.0);

        egui::Window::new(crate::lang::t(lang, "screenshot_title"))
            .id(egui::Id::new("screenshot_dialog"))
            .collapsible(false)
            .resizable(true)
            .default_pos(ctx.screen_rect().center() - default_size / 2.0)
            .default_size(default_size)
            .min_size([280.0, 190.0])
            .show(ctx, |ui| {
                self.poll_workers(ctx);

                // Report exactly the requested content bounds to egui's window resize logic.
                // Lay out the controls from the bottom so the preview uses their actual height.
                let content_rect = ui.available_rect_before_wrap();
                ui.allocate_rect(content_rect, egui::Sense::hover());
                let mut controls = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(egui::Layout::bottom_up(egui::Align::Min)),
                );
                controls.shrink_clip_rect(content_rect);

                let (close, capture) = controls
                    .horizontal(|ui| {
                        let cancel = ui.add(Icon::Close.labeled(ui, crate::lang::t(lang, "cancel"))).clicked();
                        let capture = ui
                            .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add(Icon::Camera.labeled(ui, crate::lang::t(lang, "capture"))).clicked()
                            })
                            .inner;
                        (cancel || capture, capture)
                    })
                    .inner;

                let total_mins = (self.total_duration / 60.0) as u32;
                let total_secs = (self.total_duration % 60.0) as u32;
                let mins = (self.current_time / 60.0) as u32;
                let secs = (self.current_time % 60.0) as u32;
                controls.horizontal(|ui| {
                    ui.label(format!("{:02}:{:02}", mins, secs));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{:02}:{:02}", total_mins, total_secs));
                    });
                });

                controls.spacing_mut().slider_width = content_rect.width();
                let time_changed = controls
                    .add(
                        egui::Slider::new(&mut self.current_time, 0.0..=self.total_duration)
                            .show_value(false),
                    )
                    .changed();
                if time_changed {
                    self.last_update_time = Instant::now();
                    self.preview_error = None;
                    ctx.request_repaint();
                }

                self.schedule_preview(ctx);

                let preview_rect = controls
                    .available_rect_before_wrap()
                    .intersect(content_rect);
                ui.painter()
                    .rect_filled(preview_rect, 6.0, ui.visuals().extreme_bg_color);

                if let Some(texture) = self
                    .preview_texture
                    .as_ref()
                    .filter(|_| self.preview_error.is_none())
                {
                    let size = texture.size_vec2();
                    let scale = (preview_rect.width() / size.x).min(preview_rect.height() / size.y);
                    ui.painter().image(
                        texture.id(),
                        egui::Rect::from_center_size(preview_rect.center(), size * scale),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    let mut placeholder =
                        ui.new_child(egui::UiBuilder::new().max_rect(preview_rect).layout(
                            egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        ));
                    placeholder.shrink_clip_rect(preview_rect);
                    if let Some(error) = &self.preview_error {
                        let response = placeholder.label(error.message(lang));
                        if let Some(details) = error.details() {
                            response.on_hover_text(details);
                        }
                    } else {
                        placeholder.spinner();
                    }
                }

                (close, capture)
            })
    }

    fn poll_workers(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.duration_rx {
            match rx.try_recv() {
                Ok(duration) => {
                    self.total_duration = duration;
                    self.current_time = self.current_time.min(duration);
                    self.duration_rx = None;
                }
                Err(TryRecvError::Disconnected) => self.duration_rx = None,
                Err(TryRecvError::Empty) => {}
            }
        }

        let Some(rx) = &self.preview_rx else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(PreviewError::Failed(
                "Preview worker stopped before returning a frame".into(),
            )),
        };
        self.preview_rx = None;

        match result {
            Ok(rgba) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                );
                self.preview_texture = Some(ctx.load_texture(
                    format!("preview_{}", self.file.id),
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            Err(error) => {
                log::warn!("Preview generation failed: {error:?}");
                if self.current_time == self.last_preview_time {
                    self.preview_error = Some(error);
                }
            }
        }
    }

    fn schedule_preview(&mut self, ctx: &egui::Context) {
        if self.duration_rx.is_some()
            || self.preview_rx.is_some()
            || self.current_time == self.last_preview_time
        {
            return;
        }
        let remaining = PREVIEW_DEBOUNCE.saturating_sub(self.last_update_time.elapsed());
        if !remaining.is_zero() {
            ctx.request_repaint_after(remaining);
            return;
        }

        self.last_preview_time = self.current_time;
        self.preview_error = None;
        let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() else {
            log::error!("ffmpeg not found");
            self.preview_error = Some(PreviewError::MissingFfmpeg);
            return;
        };

        let video_path = self.file.file_path.clone();
        let time = self.current_time;
        let size = self.thumbnail_size;
        let ctx = ctx.clone();

        let (tx, rx) = mpsc::channel();
        self.preview_rx = Some(rx);

        std::thread::spawn(move || {
            if tx
                .send(Self::extract_preview(&ffmpeg_path, &video_path, time, size))
                .is_ok()
            {
                ctx.request_repaint();
            }
        });
    }

    fn extract_preview(
        ffmpeg: &Path,
        video_path: &str,
        time: f64,
        [width, height]: [u32; 2],
    ) -> PreviewResult {
        if std::fs::metadata(video_path)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            return Err(PreviewError::MissingFile(video_path.into()));
        }

        let output = std::process::Command::new(ffmpeg)
            .args(["-nostdin", "-v", "error", "-ss", &format!("{time:.6}")])
            .args(["-i", video_path, "-map", "0:V:0", "-frames:v", "1"])
            .args([
                "-vf",
                &format!("scale={width}:{height}:force_original_aspect_ratio=decrease"),
            ])
            .args(["-c:v", "mjpeg", "-q:v", "2", "-f", "image2pipe", "pipe:1"])
            .output()
            .map_err(|error| PreviewError::Failed(error.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let details = if stderr.trim().is_empty() {
                format!("FFmpeg exited with {}", output.status)
            } else {
                stderr.trim().into()
            };
            return Err(PreviewError::Failed(details));
        }
        if output.stdout.is_empty() {
            return Err(PreviewError::NoFrame);
        }
        image::load_from_memory_with_format(&output.stdout, image::ImageFormat::Jpeg)
            .map(|image| image.to_rgba8())
            .map_err(|error| PreviewError::Failed(error.to_string()))
    }

    fn capture_screenshot(&mut self, app: &mut App) {
        let Some(ffmpeg_path) = crate::utils::get_ffmpeg_path() else {
            log::error!("ffmpeg not found");
            return;
        };

        let data_dir = crate::utils::get_data_dir();

        let gen = crate::thumbnail::ThumbnailGenerator::new(&data_dir, &ffmpeg_path, app.db.clone());

        if let Ok(thumb_path) = gen.generate_thumbnail_at_time(&self.file.file_path, self.current_time, Some(self.file.id)) {
            let db_result = app.db.safe_lock()
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
        Self {
            file: FileEntry {
                id: 0,
                path_id: 0,
                file_path: String::new(),
                file_name: String::new(),
                file_ext: String::new(),
                thumbnail_path: None,
                file_size: 0,
                modified_at: chrono::Utc::now(),
                rating: 0,
                has_custom_thumb: false,
                created_at: chrono::Utc::now(),
            },
            current_time: 0.0,
            total_duration: 60.0,
            preview_texture: None,
            thumbnail_size: [320, 180],
            last_update_time: Instant::now(),
            last_preview_time: -1.0,
            preview_rx: None,
            duration_rx: None,
            preview_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Lang;

    #[test]
    fn missing_source_has_a_distinct_preview_error() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("missing.mp4");
        let path = video.to_str().unwrap();
        let result = ScreenshotDialog::extract_preview(Path::new("unused"), path, 0.0, [320, 180]);
        assert!(matches!(result, Err(PreviewError::MissingFile(ref missing)) if missing == path));
    }

    #[test]
    fn debounce_requests_a_future_frame() {
        let ctx = egui::Context::default();
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |_| {});
        }
        let mut dialog = ScreenshotDialog::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            dialog.last_update_time = Instant::now();
            dialog.schedule_preview(ctx);
        });
        let delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        assert!(!delay.is_zero() && delay <= PREVIEW_DEBOUNCE, "{delay:?}");
        assert!(dialog.preview_rx.is_none());
    }

    #[test]
    #[ignore = "requires FFmpeg and FFprobe"]
    fn screenshot_shows_real_preview_on_open_and_after_seeking() {
        let ffmpeg = crate::utils::get_ffmpeg_path().expect("FFmpeg required");
        let temp = tempfile::tempdir().unwrap();
        let video = temp.path().join("preview smoke.mp4");
        let output = std::process::Command::new(ffmpeg)
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=160x120:r=10:d=0.6",
            ])
            .args(["-c:v", "mpeg4"])
            .arg(&video)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let ctx = egui::Context::default();
        let mut file = ScreenshotDialog::default().file;
        file.file_path = video.to_str().unwrap().into();
        let mut dialog = ScreenshotDialog::new(file, &ctx);

        let mut previous_texture = None;
        for time in [0.0, 0.3] {
            dialog.current_time = time;
            dialog.last_update_time = Instant::now();
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let output = ctx.run(egui::RawInput::default(), |ctx| {
                    dialog.show_window(ctx, Lang::En);
                });
                assert!(dialog.preview_error.is_none(), "{:?}", dialog.preview_error);
                if let Some(texture) = &dialog.preview_texture {
                    let id = texture.id();
                    if Some(id) != previous_texture && dialog.last_preview_time == time {
                        assert!((dialog.total_duration - 0.6).abs() < 0.001);
                        assert_eq!(texture.size(), [240, 180]);
                        let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
                        assert!(meshes.iter().any(|mesh| matches!(
                            &mesh.primitive,
                            egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id == id
                        )));
                        previous_texture = Some(id);
                        break;
                    }
                }
                assert!(Instant::now() < deadline, "Preview timed out at {time}");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    #[test]
    fn screenshot_window_keeps_size_until_resized() {
        let ctx = egui::Context::default();
        crate::setup_fonts(&ctx);
        ctx.set_zoom_factor(1.25);
        let mut dialog = ScreenshotDialog::default();
        dialog.last_preview_time = dialog.current_time;

        let mut frame = |events| {
            let mut rect = egui::Rect::NOTHING;
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    rect = dialog.show_window(ctx, Lang::ZhTw).unwrap().response.rect;
                },
            );
            rect
        };

        frame(vec![]);
        let initial = frame(vec![]);
        for _ in 0..10 {
            assert_eq!(frame(vec![]).size(), initial.size());
        }

        let pointer_button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut window = initial;
        for delta in [egui::vec2(60.0, 40.0), egui::vec2(-120.0, -80.0)] {
            let corner = window.right_bottom() - egui::vec2(1.0, 1.0);
            let target = corner + delta;
            frame(vec![egui::Event::PointerMoved(corner)]);
            frame(vec![pointer_button(corner, true)]);
            frame(vec![egui::Event::PointerMoved(target)]);
            frame(vec![pointer_button(target, false)]);
            let resized = frame(vec![]);
            assert!((resized.size() - window.size() - delta).length() < 2.0);
            for _ in 0..10 {
                assert_eq!(frame(vec![]).size(), resized.size());
            }
            window = resized;
        }
    }
}
