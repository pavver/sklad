//! The `Icon` primitive: single-colour Material Symbols Sharp icons, rasterised once per size and
//! tinted with the current content colour.

use std::collections::HashMap;

use eframe::egui::{
    Color32, ColorImage, Context, Id, Pos2, Rect, TextureHandle, TextureOptions, Ui, pos2, vec2,
};
use operator_ui_tokens::icons as ds;

/// An icon of the design system set, or one sklad imported from the same source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    System(&'static operator_ui_tokens::Icon),
    Imported(&'static str, &'static str),
}

macro_rules! imported {
    ($($constant:ident = $file:literal),* $(,)?) => {
        $(pub const $constant: Icon =
            Icon::Imported($file, include_str!(concat!("../../../assets/icons/", $file, ".svg")));)*
    };
}

pub const ARROW_DOWNWARD: Icon = Icon::System(&ds::ARROW_DOWNWARD);
pub const ARROW_DROP_DOWN: Icon = Icon::System(&ds::ARROW_DROP_DOWN);
pub const ARROW_UPWARD: Icon = Icon::System(&ds::ARROW_UPWARD);
pub const CHECK: Icon = Icon::System(&ds::CHECK);
pub const CHECK_CIRCLE: Icon = Icon::System(&ds::CHECK_CIRCLE);
pub const CLOSE: Icon = Icon::System(&ds::CLOSE);
pub const CONTENT_COPY: Icon = Icon::System(&ds::CONTENT_COPY);
pub const DELETE: Icon = Icon::System(&ds::DELETE);
pub const ERROR: Icon = Icon::System(&ds::ERROR);
pub const INBOX: Icon = Icon::System(&ds::INBOX);
pub const REFRESH: Icon = Icon::System(&ds::REFRESH);
pub const SEARCH: Icon = Icon::System(&ds::SEARCH);
pub const SETTINGS: Icon = Icon::System(&ds::SETTINGS);
pub const WARNING: Icon = Icon::System(&ds::WARNING);

imported!(
    ALL_INCLUSIVE = "all_inclusive",
    APPS = "apps",
    BUILD = "build",
    CODE = "code",
    CONSTRUCTION = "construction",
    DEPLOYED_CODE = "deployed_code",
    DESCRIPTION = "description",
    EXTENSION = "extension",
    FOLDER = "folder",
    FONT_DOWNLOAD = "font_download",
    GROUPS = "groups",
    INVENTORY_2 = "inventory_2",
    LAYERS = "layers",
    MEMORY = "memory",
    MOVIE = "movie",
    OPEN_IN_NEW = "open_in_new",
    PALETTE = "palette",
    PLAY_ARROW = "play_arrow",
    PUBLIC = "public",
    SCHOOL = "school",
    SCIENCE = "science",
    SPORTS_ESPORTS = "sports_esports",
    TERMINAL = "terminal",
);

impl Icon {
    pub fn name(self) -> &'static str {
        match self {
            Icon::System(icon) => icon.name,
            Icon::Imported(name, _) => name,
        }
    }

    /// The icon as an SVG document filled in white, so that a tint gives it any colour.
    fn svg(self) -> String {
        match self {
            Icon::System(icon) => {
                let paths: String = icon
                    .paths
                    .iter()
                    .map(|path| format!("<path d=\"{path}\"/>"))
                    .collect();
                format!(
                    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{}\" fill=\"#ffffff\">{paths}</svg>",
                    icon.view_box
                )
            }
            Icon::Imported(_, svg) => svg.replacen("<svg ", "<svg fill=\"#ffffff\" ", 1),
        }
    }
}

/// Renders an SVG document into a square of `pixels`, RGBA with premultiplied alpha.
pub fn rasterise(svg: &str, pixels: u32) -> Option<ColorImage> {
    let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &resvg::usvg::Options::default()).ok()?;
    rasterise_tree(&tree, pixels, pixels)
}

pub fn rasterise_tree(tree: &resvg::usvg::Tree, width: u32, height: u32) -> Option<ColorImage> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width.max(1), height.max(1))?;
    let source = tree.size();
    let scale = (width as f32 / source.width()).min(height as f32 / source.height());
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(tree, transform, &mut pixmap.as_mut());
    let size = [pixmap.width() as usize, pixmap.height() as usize];
    Some(ColorImage::from_rgba_premultiplied(size, pixmap.data()))
}

#[derive(Clone, Default)]
struct Textures(HashMap<(&'static str, u32), Option<TextureHandle>>);

fn texture(ctx: &Context, icon: Icon, pixels: u32) -> Option<TextureHandle> {
    let key = (icon.name(), pixels);
    let id = Id::unique("operator icons");
    if let Some(found) = ctx.data(|data| data.get_temp::<Textures>(id).and_then(|t| t.0.get(&key).cloned())) {
        return found;
    }
    let made = rasterise(&icon.svg(), pixels).map(|image| {
        ctx.load_texture(
            format!("icon {} {pixels}", icon.name()),
            image,
            TextureOptions::LINEAR,
        )
    });
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Textures>(id)
            .0
            .insert(key, made.clone());
    });
    made
}

/// Paints `icon` centred on `centre`, `size` points on a side, in `colour`.
pub fn paint_icon(ui: &Ui, icon: Icon, centre: Pos2, size: f32, colour: Color32) {
    let pixels = (size * ui.ctx().pixels_per_point()).round().max(1.0) as u32;
    if let Some(texture) = texture(ui.ctx(), icon, pixels) {
        let rect = Rect::from_center_size(centre, vec2(size, size));
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter().image(texture.id(), rect, uv, colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_draws_something() {
        let all = [
            ARROW_DOWNWARD,
            ARROW_UPWARD,
            CHECK,
            CHECK_CIRCLE,
            CLOSE,
            CONTENT_COPY,
            DELETE,
            ERROR,
            INBOX,
            REFRESH,
            SEARCH,
            SETTINGS,
            WARNING,
            ALL_INCLUSIVE,
            APPS,
            BUILD,
            CODE,
            CONSTRUCTION,
            DEPLOYED_CODE,
            DESCRIPTION,
            EXTENSION,
            FOLDER,
            FONT_DOWNLOAD,
            GROUPS,
            INVENTORY_2,
            LAYERS,
            MEMORY,
            MOVIE,
            OPEN_IN_NEW,
            PALETTE,
            PUBLIC,
            SCHOOL,
            SCIENCE,
            SPORTS_ESPORTS,
            TERMINAL,
        ];
        for icon in all {
            let image = rasterise(&icon.svg(), 32);
            let inked = image.is_some_and(|image| image.pixels.iter().any(|pixel| pixel.a() > 0));
            assert!(inked, "{} is empty", icon.name());
        }
    }
}
