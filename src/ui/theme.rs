use egui::{epaint::Shadow, Color32, Stroke, Theme, Visuals};

pub(crate) fn visuals(theme: Theme) -> Visuals {
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
