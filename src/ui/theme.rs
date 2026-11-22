use egui::{epaint::Shadow, Color32, Stroke, Theme, Visuals};

// Preserve the preference already saved by v0.1.23.
pub(crate) const PLATINUM_SETTING: &str = "retro";

pub(crate) fn startup_setting(saved: Option<String>) -> String {
    saved.unwrap_or_else(|| PLATINUM_SETTING.to_owned())
}

pub(crate) fn apply(ctx: &egui::Context, setting: &str) {
    let theme = match setting {
        "light" | PLATINUM_SETTING => Theme::Light,
        _ => Theme::Dark,
    };
    let mut style = egui::Style {
        visuals: visuals(theme),
        ..Default::default()
    };
    if setting == PLATINUM_SETTING {
        style.visuals = platinum_visuals();
        style.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 15.0,
            foreground_color: false,
            ..egui::style::ScrollStyle::solid()
        };
    }
    ctx.set_style_of(theme, style);
    ctx.set_theme(theme);
    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(
        setting != PLATINUM_SETTING,
    ));
    ctx.request_repaint();
}

pub(super) fn platinum_title_bar(ctx: &egui::Context, lang: crate::lang::Lang) {
    use egui::ViewportCommand;

    let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
    let toggle_size = ViewportCommand::Maximized(!maximized);
    egui::TopBottomPanel::top("platinum_title_bar")
        .exact_height(30.0)
        .frame(
            egui::Frame::new()
                .fill(Color32::from_gray(204))
                .inner_margin(5),
        )
        .show(ctx, |ui| {
            let bar = ui.available_rect_before_wrap();
            let size = egui::vec2(19.0, 19.0);
            let close =
                egui::Rect::from_center_size(bar.left_center() + egui::vec2(10.0, 0.0), size);
            let zoom =
                egui::Rect::from_center_size(bar.right_center() - egui::vec2(10.0, 0.0), size);
            let minimize = zoom.translate(egui::vec2(-24.0, 0.0));
            let title = bar.shrink2(egui::vec2(48.0, 0.0));
            let drag = ui.interact(title, ui.id().with("drag"), egui::Sense::click_and_drag());
            if drag.double_clicked() {
                ctx.send_viewport_cmd(toggle_size.clone());
            } else if drag.drag_started_by(egui::PointerButton::Primary) {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }
            let painter = ui.painter().with_clip_rect(title.intersect(ui.clip_rect()));
            for offset in [-6.0, -2.0, 2.0, 6.0] {
                let y = title.center().y + offset;
                painter.hline(title.x_range(), y, Stroke::new(1.0_f32, Color32::GRAY));
                painter.hline(
                    title.x_range(),
                    y + 1.0,
                    Stroke::new(1.0_f32, Color32::WHITE),
                );
            }
            let ink = ui.visuals().text_color();
            let label = painter.layout_no_wrap(
                crate::lang::t(lang, "app_name").to_owned(),
                egui::FontId::proportional(14.0),
                ink,
            );
            let label_rect = egui::Rect::from_center_size(title.center(), label.size());
            painter.rect_filled(
                label_rect.expand2(egui::vec2(7.0, 0.0)),
                0,
                Color32::from_gray(204),
            );
            painter.galley(label_rect.min, label, ink);
            for (rect, mark, key, command) in [
                (close, "×", "window_close", ViewportCommand::Close),
                (
                    minimize,
                    "−",
                    "window_minimize",
                    ViewportCommand::Minimized(true),
                ),
                (
                    zoom,
                    if maximized { "❐" } else { "□" },
                    if maximized {
                        "window_restore"
                    } else {
                        "window_maximize"
                    },
                    toggle_size,
                ),
            ] {
                let label = crate::lang::t(lang, key);
                let response = ui
                    .put(rect, egui::Button::new(mark).min_size(size))
                    .on_hover_text(label);
                if response.clicked() {
                    ctx.send_viewport_cmd(command);
                }
            }
        });
}

pub(super) fn platinum_window_resize(ctx: &egui::Context) {
    use egui::{CursorIcon, ResizeDirection, ViewportCommand};

    let pointer = ctx.input(|input| {
        let viewport = input.viewport();
        if viewport.maximized.unwrap_or(false) || viewport.fullscreen.unwrap_or(false) {
            return None;
        }
        input.pointer.hover_pos().map(|position| {
            (
                position,
                input.screen_rect(),
                input.pointer.button_pressed(egui::PointerButton::Primary),
            )
        })
    });
    let Some((pos, rect, pressed)) = pointer else {
        return;
    };
    let west = pos.x - rect.left() < 4.0;
    let east = rect.right() - pos.x < 4.0;
    let north = pos.y - rect.top() < 4.0;
    let south = rect.bottom() - pos.y < 4.0;
    let (direction, cursor) = match (west, east, north, south) {
        (true, _, true, _) => (ResizeDirection::NorthWest, CursorIcon::ResizeNwSe),
        (true, _, _, true) => (ResizeDirection::SouthWest, CursorIcon::ResizeNeSw),
        (_, true, true, _) => (ResizeDirection::NorthEast, CursorIcon::ResizeNeSw),
        (_, true, _, true) => (ResizeDirection::SouthEast, CursorIcon::ResizeNwSe),
        (true, _, _, _) => (ResizeDirection::West, CursorIcon::ResizeHorizontal),
        (_, true, _, _) => (ResizeDirection::East, CursorIcon::ResizeHorizontal),
        (_, _, true, _) => (ResizeDirection::North, CursorIcon::ResizeVertical),
        (_, _, _, true) => (ResizeDirection::South, CursorIcon::ResizeVertical),
        _ => return,
    };
    ctx.set_cursor_icon(cursor);
    if pressed {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}

fn platinum_visuals() -> Visuals {
    let mut visuals = Visuals::light();
    let ink = Color32::from_gray(55);
    let border = Stroke::new(1.0_f32, ink);
    visuals.panel_fill = Color32::from_gray(204);
    visuals.window_fill = Color32::from_gray(221);
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.faint_bg_color = Color32::from_gray(245);
    visuals.window_stroke = border;
    visuals.window_corner_radius = 0.into();
    visuals.menu_corner_radius = 0.into();
    visuals.window_shadow = Shadow {
        offset: [1, 2],
        blur: 0,
        spread: 0,
        color: Color32::from_black_alpha(100),
    };
    visuals.popup_shadow = visuals.window_shadow;
    visuals.selection.bg_fill = Color32::from_rgb(198, 192, 226);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(96, 86, 145));
    for (widget, fill) in [
        (&mut visuals.widgets.noninteractive, visuals.panel_fill),
        (&mut visuals.widgets.inactive, Color32::from_gray(216)),
        (
            &mut visuals.widgets.hovered,
            Color32::from_rgb(215, 209, 236),
        ),
        (&mut visuals.widgets.active, visuals.selection.bg_fill),
        (&mut visuals.widgets.open, visuals.selection.bg_fill),
    ] {
        widget.corner_radius = 0.into();
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = border;
        widget.fg_stroke.color = ink;
    }
    visuals
}

pub(super) fn platinum_browser<T>(
    ui: &mut egui::Ui,
    title: &str,
    content: impl FnOnce(&mut egui::Ui) -> T,
) -> T {
    let frame = egui::Frame::new()
        .outer_margin(4)
        .inner_margin(4)
        .fill(ui.visuals().panel_fill)
        .stroke(ui.visuals().window_stroke)
        .show(ui, |ui| {
            let (header, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 21.0), egui::Sense::hover());
            let painter = ui
                .painter()
                .with_clip_rect(header.intersect(ui.clip_rect()));
            for offset in [4.0, 7.0, 10.0, 13.0, 16.0] {
                let y = header.top() + offset;
                painter.hline(
                    header.x_range(),
                    y,
                    Stroke::new(1.0_f32, Color32::from_gray(149)),
                );
                painter.hline(
                    header.x_range(),
                    y + 1.0,
                    Stroke::new(1.0_f32, Color32::from_gray(238)),
                );
            }
            let ink = ui.visuals().text_color();
            let mut job = egui::text::LayoutJob::simple(
                title.to_owned(),
                egui::FontId::proportional(14.0),
                ink,
                (header.width() - 24.0).max(0.0),
            );
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            let label = painter.layout_job(job);
            let label_rect = egui::Rect::from_center_size(header.center(), label.size());
            painter.rect_filled(
                label_rect.expand2(egui::vec2(7.0, 0.0)),
                0,
                ui.visuals().panel_fill,
            );
            painter.galley(label_rect.min, label, ink);
            egui::Frame::new()
                .inner_margin(6)
                .fill(ui.visuals().extreme_bg_color)
                .stroke(ui.visuals().window_stroke)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    content(ui)
                })
                .inner
        });
    let bevel = frame.response.rect.shrink(5.5);
    let highlight = Stroke::new(1.0_f32, Color32::WHITE);
    let shadow = Stroke::new(1.0_f32, Color32::from_gray(128));
    ui.painter().hline(bevel.x_range(), bevel.top(), highlight);
    ui.painter().vline(bevel.left(), bevel.y_range(), highlight);
    ui.painter().hline(bevel.x_range(), bevel.bottom(), shadow);
    ui.painter().vline(bevel.right(), bevel.y_range(), shadow);
    frame.inner
}

fn visuals(theme: Theme) -> Visuals {
    let rgb = Color32::from_rgb;
    let (mut visuals, text, border, accent, selection, control, hovered, pressed) = match theme {
        Theme::Light => {
            let mut visuals = Visuals::light();
            visuals.panel_fill = rgb(225, 223, 215);
            visuals.window_fill = rgb(244, 241, 231);
            visuals.window_stroke = Stroke::new(1.5_f32, rgb(107, 112, 106));
            visuals.faint_bg_color = rgb(234, 231, 222);
            visuals.extreme_bg_color = rgb(216, 216, 207);
            visuals.warn_fg_color = rgb(124, 79, 14);
            (
                visuals,
                rgb(39, 49, 51),
                rgb(177, 181, 173),
                rgb(24, 99, 93),
                rgb(179, 211, 202),
                rgb(213, 219, 209),
                rgb(199, 218, 209),
                rgb(184, 207, 197),
            )
        }
        Theme::Dark => {
            let mut visuals = Visuals::dark();
            visuals.panel_fill = rgb(20, 27, 40);
            visuals.window_fill = rgb(39, 54, 74);
            visuals.window_stroke = Stroke::new(1.5_f32, rgb(104, 140, 166));
            visuals.faint_bg_color = rgb(28, 38, 54);
            visuals.extreme_bg_color = rgb(14, 20, 31);
            visuals.warn_fg_color = rgb(246, 194, 106);
            (
                visuals,
                rgb(226, 234, 245),
                rgb(61, 80, 106),
                rgb(98, 220, 201),
                rgb(34, 81, 87),
                rgb(41, 54, 75),
                rgb(48, 70, 94),
                rgb(50, 89, 108),
            )
        }
    };

    visuals.code_bg_color = control;
    let shadow_color = Color32::from_black_alpha(if visuals.dark_mode { 175 } else { 85 });
    visuals.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 2,
        color: shadow_color,
    };
    visuals.popup_shadow = Shadow {
        offset: [0, 5],
        blur: 16,
        spread: 1,
        color: shadow_color,
    };
    visuals.window_corner_radius = 10.into();
    visuals.menu_corner_radius = 8.into();
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = selection;
    visuals.selection.stroke = Stroke::new(1.0_f32, accent);
    visuals.widgets.noninteractive.bg_stroke.color = border;
    visuals.widgets.noninteractive.fg_stroke.color = text;

    for (widget, fill, outline) in [
        (&mut visuals.widgets.inactive, control, border),
        (&mut visuals.widgets.hovered, hovered, accent),
        (&mut visuals.widgets.active, pressed, accent),
        (&mut visuals.widgets.open, hovered, accent),
    ] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke.color = outline;
        widget.fg_stroke.color = text;
    }
    visuals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_defaults_to_platinum_and_keeps_saved_preferences() {
        assert_eq!(startup_setting(None), PLATINUM_SETTING);
        for saved in ["light", "dark", PLATINUM_SETTING] {
            assert_eq!(startup_setting(Some(saved.to_owned())), saved);
        }
    }

    #[test]
    fn platinum_browser_uses_silver_chrome_and_restores_light_style() {
        let ctx = egui::Context::default();
        apply(&ctx, "light");
        let light_style = ctx.style_of(Theme::Light);
        apply(&ctx, PLATINUM_SETTING);
        let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 480.0));
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(bounds),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        let panel = ui.max_rect();
                        platinum_browser(ui, "Videos", |ui| {
                            assert!(panel.shrink(4.0).contains_rect(ui.max_rect()));
                            assert_eq!(ui.visuals().panel_fill, Color32::from_gray(204));
                            assert_eq!(ui.visuals().extreme_bg_color, Color32::WHITE);
                            assert!(ui.available_width() > 500.0);
                            assert!(ui.available_height() > 300.0);
                            assert!(!ui.spacing().scroll.floating);
                        });
                    });
            },
        );
        apply(&ctx, "light");
        let restored = ctx.style_of(Theme::Light);
        assert_eq!(
            restored.visuals.window_corner_radius,
            light_style.visuals.window_corner_radius
        );
        assert_eq!(restored.spacing.scroll, light_style.spacing.scroll);
    }
}
