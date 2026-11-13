//! `ThemeToggle`: the switch between the light and the dark theme.

use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, Id, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    TextureOptions, Ui, WidgetInfo, pos2, vec2,
};

use super::icon::{self};
use super::motion::{cubic_bezier, smooth, standard};
use super::state::{BORDER, paint_focus_ring};
use super::tokens::{Palette, du, mix, secs, tint};

const CLOUD_BACK: &str = "M335 147.5c0 27.89-22.61 50.5-50.5 50.5a50.78 50.78 0 0 1-9.29-.853c-2.478 12.606-10.595 23.188-21.615 29.011C245.699 243.749 228.03 256 207.5 256a50.433 50.433 0 0 1-16.034-2.599A41.811 41.811 0 0 1 166 262a41.798 41.798 0 0 1-22.893-6.782A42.21 42.21 0 0 1 135 256a41.82 41.82 0 0 1-19.115-4.592A41.84 41.84 0 0 1 88 262c-1.827 0-3.626-.117-5.391-.343C74.911 270.448 63.604 276 51 276c-23.196 0-42-18.804-42-42s18.804-42 42-42c1.827 0 3.626.117 5.391.343C64.089 183.552 75.396 178 88 178a41.819 41.819 0 0 1 19.115 4.592C114.532 176.002 124.298 172 135 172a41.798 41.798 0 0 1 22.893 6.782 42.066 42.066 0 0 1 7.239-.773C174.137 164.159 189.749 155 207.5 155c.601 0 1.199.01 1.794.031A41.813 41.813 0 0 1 234 147h.002c.269-27.66 22.774-50 50.498-50 27.89 0 50.5 22.61 50.5 50.5Z";
const CLOUD_FRONT: &str = "M328 167.5c0 15.214-7.994 28.56-20.01 36.068.007.31.01.621.01.932 0 23.472-19.028 42.5-42.5 42.5-3.789 0-7.461-.496-10.957-1.426C249.671 263.676 233.141 277 213.5 277a42.77 42.77 0 0 1-7.702-.696C198.089 284.141 187.362 289 175.5 289a42.338 42.338 0 0 1-27.864-10.408A42.411 42.411 0 0 1 133.5 281c-4.36 0-8.566-.656-12.526-1.876C113.252 287.066 102.452 292 90.5 292a42.388 42.388 0 0 1-15.8-3.034A42.316 42.316 0 0 1 48.5 298C25.028 298 6 278.972 6 255.5S25.028 213 48.5 213a42.388 42.388 0 0 1 15.8 3.034A42.316 42.316 0 0 1 90.5 207c4.36 0 8.566.656 12.526 1.876C110.748 200.934 121.548 196 133.5 196a42.338 42.338 0 0 1 27.864 10.408A42.411 42.411 0 0 1 175.5 204c2.63 0 5.204.239 7.702.696C190.911 196.859 201.638 192 213.5 192c3.789 0 7.461.496 10.957 1.426 2.824-10.491 9.562-19.377 18.553-24.994-.007-.31-.01-.621-.01-.932 0-23.472 19.028-42.5 42.5-42.5s42.5 19.028 42.5 42.5Z";

/// Stars as (x ratio, y ratio, radius in DU).
const STARS: [(f32, f32, f32); 7] = [
    (0.16, 0.22, 0.55),
    (0.28, 0.35, 0.375),
    (0.14, 0.68, 0.45),
    (0.24, 0.82, 0.35),
    (0.38, 0.25, 0.3),
    (0.42, 0.70, 0.4),
    (0.50, 0.38, 0.5),
];

fn hex(colour: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", colour.r(), colour.g(), colour.b())
}

/// The picture of the toggle at a point of its movement, as SVG in logical points.
fn toggle_svg(
    width: f32,
    height: f32,
    travel: f32,
    colours: [Color32; 2],
    night: f32,
    palette: &Palette,
) -> String {
    let [sky, orb] = colours;
    let pad = du(0.625);
    let d = height - 2.0 * pad;
    let r = d / 2.0;
    let slide = (width - d - 2.0 * pad) * travel;
    let cloud_scale = width / 290.0;
    let cloud_y = height / cloud_scale - 228.0 + 160.0 * night;
    let cloud_opacity = (1.0 - night).clamp(0.0, 1.0);
    let star_scale = 0.3 + 0.7 * night;
    let star_opacity = (0.95 * night).clamp(0.0, 0.95);
    let stars: String = STARS
        .iter()
        .map(|&(x, y, radius)| {
            let (x, y, r) = (width * x, height * y, du(radius) * star_scale);
            format!(
                "<path d=\"M{x} {t}Q{x} {y} {rr} {y}Q{x} {y} {x} {b}Q{x} {y} {l} {y}Q{x} {y} {x} {t}Z\"/>",
                t = y - r,
                b = y + r,
                l = x - r,
                rr = x + r
            )
        })
        .collect();
    let halo = (1.0 - travel).clamp(0.0, 1.0);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\
         <defs><clipPath id=\"track\"><rect width=\"{width}\" height=\"{height}\" rx=\"{rx}\"/></clipPath></defs>\
         <rect width=\"{width}\" height=\"{height}\" rx=\"{rx}\" fill=\"{sky}\"/>\
         <g clip-path=\"url(#track)\">\
         <g fill=\"{star}\" opacity=\"{star_opacity}\">{stars}</g>\
         <g transform=\"scale({cloud_scale}) translate(0 {cloud_y})\" opacity=\"{cloud_opacity}\">\
         <path d=\"{CLOUD_BACK}\" transform=\"translate(0 -10)\" fill=\"{cloud_back}\" fill-opacity=\"0.45\"/>\
         <path d=\"{CLOUD_FRONT}\" fill=\"{cloud}\" fill-opacity=\"0.95\"/></g>\
         <g transform=\"translate({tx} {pad})\">\
         <circle cx=\"{r}\" cy=\"{r}\" r=\"{h1}\" fill=\"{star}\" opacity=\"{o1}\"/>\
         <circle cx=\"{r}\" cy=\"{r}\" r=\"{h2}\" fill=\"{star}\" opacity=\"{o2}\"/>\
         <circle cx=\"{r}\" cy=\"{r}\" r=\"{r}\" fill=\"{orb}\"/>\
         <g fill=\"{crater}\" opacity=\"{craters}\">\
         <circle cx=\"{c1x}\" cy=\"{c1y}\" r=\"{c1r}\"/><circle cx=\"{c2x}\" cy=\"{c2y}\" r=\"{c2r}\"/>\
         <circle cx=\"{c3x}\" cy=\"{c3y}\" r=\"{c3r}\"/></g></g></g></svg>",
        rx = height / 2.0,
        sky = hex(sky),
        star = hex(palette.toggle_star),
        cloud_back = hex(palette.toggle_cloud_back),
        cloud = hex(palette.toggle_cloud),
        tx = pad + slide,
        h1 = r * 2.1,
        h2 = r * 1.55,
        o1 = 0.12 * halo,
        o2 = 0.22 * halo,
        orb = hex(orb),
        crater = hex(palette.toggle_crater),
        craters = night.clamp(0.0, 1.0),
        c1x = d * 0.42,
        c1y = d * 0.26,
        c1r = r * 0.22,
        c2x = d * 0.68,
        c2y = d * 0.62,
        c2r = r * 0.25,
        c3x = d * 0.28,
        c3y = d * 0.52,
        c3r = r * 0.35,
    )
}

#[derive(Clone)]
struct ToggleCache {
    svg: String,
    pixels: [u32; 2],
    texture: Option<TextureHandle>,
}

/// `ThemeToggle`: the day and night switch. Returns whether it was clicked.
pub fn theme_toggle(ui: &mut Ui, id: Id, dark: bool, label: &str, palette: &Palette) -> bool {
    let size = vec2(du(17.0), du(7.5));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::Role::Switch, true, dark, label));
    let ctx = ui.ctx().clone();
    let time = secs(450);
    let travel = ctx.animate_bool_with_time_and_easing(id.with("travel"), dark, time, |t| {
        cubic_bezier(0.4, -0.2, 0.4, 1.2, t)
    });
    let sky = ctx.animate_bool_with_time_and_easing(id.with("sky"), dark, secs(400), standard);
    let body = ctx.animate_bool_with_time_and_easing(id.with("body"), dark, time, smooth);
    let colours = [
        mix(palette.toggle_day_sky, palette.toggle_night_sky, sky),
        mix(palette.toggle_sun, palette.toggle_moon, body),
    ];
    let svg = toggle_svg(size.x, size.y, travel, colours, travel.clamp(0.0, 1.0), palette);
    let ppp = ctx.pixels_per_point();
    let pixels = [(size.x * ppp).round() as u32, (size.y * ppp).round() as u32];
    let key = id.with("picture");
    let mut cache: Option<ToggleCache> = ctx.data(|data| data.get_temp(key));
    let stale = cache
        .as_ref()
        .is_none_or(|cache| cache.svg != svg || cache.pixels != pixels);
    if stale {
        let texture = resvg::usvg::Tree::from_data(svg.as_bytes(), &resvg::usvg::Options::default())
            .ok()
            .and_then(|tree| icon::rasterise_tree(&tree, pixels[0], pixels[1]))
            .map(|image| ctx.load_texture("theme toggle", image, TextureOptions::LINEAR));
        cache = Some(ToggleCache { svg, pixels, texture });
    }
    if let Some(cache) = &cache
        && let Some(texture) = &cache.texture
    {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
    }
    ctx.data_mut(|data| data.insert_temp(key, cache));
    let pressed = response.is_pointer_button_down_on();
    let outline = if pressed {
        palette.border_strong
    } else {
        tint(palette.border, 0.5)
    };
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same((size.y / 2.0) as u8),
        Stroke::new(BORDER, outline),
        StrokeKind::Inside,
    );
    paint_focus_ring(ui, &response, rect, palette);
    response.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_toggle_picture_is_valid_svg_in_both_modes() {
        let palette = super::super::tokens::DARK;
        for night in [0.0, 1.0] {
            let svg = toggle_svg(
                68.0,
                30.0,
                night,
                [palette.toggle_day_sky, palette.toggle_sun],
                night,
                &palette,
            );
            assert!(resvg::usvg::Tree::from_data(svg.as_bytes(), &resvg::usvg::Options::default()).is_ok());
        }
    }
}
