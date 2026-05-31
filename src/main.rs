mod app;
mod config;
mod db;
mod lang;
mod scanner;
mod thumbnail;
mod ui;
mod utils;

#[cfg(target_os = "windows")]
fn toggle_console_window(show: bool) {
    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hwnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
        fn GetConsoleWindow() -> *mut std::ffi::c_void;
    }

    const SW_HIDE: i32 = 0;
    const SW_SHOW: i32 = 5;

    unsafe {
        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            ShowWindow(hwnd, if show { SW_SHOW } else { SW_HIDE });
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn toggle_console_window(_show: bool) {}

use app::App;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
use eframe::egui;
use log::{error, info, warn};
use std::panic;

fn main() -> Result<(), eframe::Error> {
    // Initialize logger first
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("Starting Video Thumb Viewer");

    // Set panic hook to log panics
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
        let message = panic_info.payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "Unknown panic".to_string());
        error!("PANIC at {}: {}", location, message);
    }));

    // Load window state from config
    let config = config::Config::load();
    let window_state = config.get_window_state();

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([window_state.width, window_state.height]);

    let viewport = if let (Some(x), Some(y)) = (window_state.x, window_state.y) {
        viewport.with_position(egui::Pos2::new(x, y))
    } else {
        viewport
    };

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        &format!("Video Thumb Viewer v{}", VERSION),
        native_options,
        Box::new(|cc| {
            info!("Setting up fonts...");
            setup_fonts(&cc.egui_ctx);

            info!("Creating app...");
            let app = App::new(cc);
            info!("App created successfully");
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    info!("Loading fonts...");

    let mut fonts = egui::FontDefinitions::default();

    // Try to load Noto Sans TC font
    match std::fs::read("assets/NotoSansTC-Regular.otf") {
        Ok(font_data) => {
            info!("Font file loaded: {} bytes", font_data.len());
            fonts.font_data.insert(
                "NotoSansTC".to_string(),
                egui::FontData::from_owned(font_data).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "NotoSansTC".to_string());
            info!("Font registered as NotoSansTC");
        }
        Err(e) => {
            warn!("Failed to load font from file: {}", e);
            // Try include_bytes as fallback
            fonts.font_data.insert(
                "NotoSansTC".to_string(),
                egui::FontData::from_owned(include_bytes!("../assets/NotoSansTC-Regular.otf").to_vec()).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "NotoSansTC".to_string());
        }
    }

    ctx.set_fonts(fonts);
    info!("Fonts setup complete");
}