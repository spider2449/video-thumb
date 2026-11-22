use eframe::egui;

const LOGO: &[u8] = include_bytes!("../../assets/logo.png");
const ATLAS: &[u8] = include_bytes!("../../assets/ui-icons.png");
const ATLAS_COLUMNS: usize = 8;
const ATLAS_ROWS: usize = 3;

pub(crate) fn window_icon() -> egui::IconData {
    let logo = decode_png(LOGO);
    egui::IconData {
        width: logo.width(),
        height: logo.height(),
        rgba: logo.into_raw(),
    }
}

pub(crate) fn logo(ui: &egui::Ui) -> egui::Image<'static> {
    let id = texture(ui.ctx(), "video-viewer-logo", LOGO);
    egui::Image::new((id, egui::vec2(26.0, 26.0)))
}

// The order matches the sprites in assets/ui-icons.png, left to right.
#[derive(Clone, Copy)]
pub(super) enum Icon {
    FolderPlus,
    Refresh,
    Trash,
    Sort,
    Search,
    Star,
    Tag,
    TagPlus,
    Moon,
    Sun,
    List,
    Grid,
    Camera,
    Folder,
    Copy,
    Play,
    Close,
    Save,
    Edit,
    Settings,
    Busy,
    Plus,
    Check,
}

impl Icon {
    pub(super) fn image(self, ui: &egui::Ui) -> egui::Image<'static> {
        let id = texture(ui.ctx(), "video-viewer-icons", ATLAS);
        let index = self as usize;
        let x = (index % ATLAS_COLUMNS) as f32;
        let y = (index / ATLAS_COLUMNS) as f32;
        let columns = ATLAS_COLUMNS as f32;
        let rows = ATLAS_ROWS as f32;
        let uv = egui::Rect::from_min_max(
            egui::pos2(x / columns, y / rows),
            egui::pos2((x + 1.0) / columns, (y + 1.0) / rows),
        );
        egui::Image::new((id, egui::vec2(18.0, 18.0))).uv(uv)
    }

    pub(super) fn button(self, ui: &egui::Ui) -> egui::Button<'static> {
        egui::Button::image(self.image(ui))
            .image_tint_follows_text_color(true)
            .min_size(egui::vec2(28.0, 26.0))
    }

    pub(super) fn labeled<'a>(
        self,
        ui: &egui::Ui,
        label: impl Into<egui::WidgetText>,
    ) -> egui::Button<'a> {
        egui::Button::image_and_text(self.image(ui), label).image_tint_follows_text_color(true)
    }
}

fn texture(ctx: &egui::Context, name: &'static str, png: &[u8]) -> egui::TextureId {
    let key = egui::Id::new(name);
    if let Some(handle) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(key)) {
        return handle.id();
    }

    let rgba = decode_png(png);
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        &rgba,
    );
    let handle = ctx.load_texture(name, image, egui::TextureOptions::LINEAR);
    let id = handle.id();
    ctx.data_mut(|data| data.insert_temp(key, handle));
    id
}

fn decode_png(bytes: &[u8]) -> image::RgbaImage {
    image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .expect("embedded UI asset must be a valid PNG")
        .into_rgba8()
}
