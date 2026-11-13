//! Content and layout components: `Avatar`, `Chip`, `PropertyList` (draft), `AsyncCheckbox`
//! (synchronous use), `ThemeToggle` and `SplitPane`.

use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, Id, Key, Rect, Response, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::button::{Fills, IconButton, paint_body};
use super::icon::{self, paint_icon};
use super::motion::{cubic_bezier, smooth, standard};
use super::state::{
    Activity, BORDER, BORDER_THICK, ControlState, Look, Outcome, Outline, hover_amount, paint_border,
    paint_focus_ring, paint_spinner, result_hold,
};
use super::text::{galley, paint_galley, paint_selectable, selectable_sense, wrapped};
use super::tokens::{BOLD, MEDIUM, Palette, Type, dim, du, mix, secs, text, tint, weighted};

/// What an avatar can show of its image.
pub enum Picture<'a> {
    Ready(&'a TextureHandle),
    Loading,
    Missing,
    /// An icon in place of the initial, for things that have no image but a kind: an agreed
    /// departure from the specification, which knows only the image and the initial.
    Glyph(icon::Icon),
}

/// The colour of an avatar without an image, from its id: Java `hashCode`, a hue out of seven,
/// HSV with the saturation and value of the mode.
pub fn avatar_colour(id: &str, dark: bool) -> Color32 {
    let hash = id.encode_utf16().fold(0_i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    });
    let hues = [190.0, 210.0, 250.0, 290.0, 330.0, 160.0, 35.0];
    let hue = hues[(hash.unsigned_abs() % 7) as usize];
    let (saturation, value) = if dark { (0.45, 0.75) } else { (0.65, 0.8) };
    hsv(hue, saturation, value)
}

fn hsv(hue: f32, saturation: f32, value: f32) -> Color32 {
    let chroma = value * saturation;
    let sector = hue / 60.0;
    let x = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = value - chroma;
    let channel = |c: f32| ((c + m) * 255.0).round() as u8;
    Color32::from_rgb(channel(r), channel(g), channel(b))
}

/// The share of the avatar a glyph fills: an icon carries less ink than a letter, so it is drawn
/// larger than the `45%` initial.
const GLYPH_SHARE: f32 = 0.6;

/// `Avatar` of the `ThreadedGroup` kind: a square with the image, or the initial on its colour.
pub fn avatar(ui: &Ui, rect: Rect, id: &str, name: &str, picture: Picture<'_>, palette: &Palette) {
    let painter = ui.painter();
    if let Picture::Ready(texture) = picture {
        let size = texture.size_vec2();
        let scale = (rect.width() / size.x).max(rect.height() / size.y);
        let shown = rect.size() / (size * scale);
        let uv = Rect::from_center_size(pos2(0.5, 0.5), shown);
        painter.image(texture.id(), rect, uv, Color32::WHITE);
    } else {
        let colour = avatar_colour(id, palette.dark);
        painter.rect_filled(rect, 0.0, tint(colour, 0.15));
        if let Picture::Glyph(glyph) = picture {
            paint_icon(ui, glyph, rect.center(), rect.height() * GLYPH_SHARE, colour);
            painter.rect_stroke(rect, 0.0, Stroke::new(BORDER, palette.border), StrokeKind::Inside);
            return;
        }
        let initial: String = name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default();
        let size = rect.height() * 0.45;
        let letter = galley(
            ui,
            egui::RichText::new(initial)
                .size(size)
                .color(colour)
                .variation("wght", f32::from(BOLD)),
        );
        paint_galley(ui, rect, letter, colour);
        if matches!(picture, Picture::Loading) {
            paint_spinner(
                ui,
                rect.center(),
                rect.height() * 0.8,
                du(dim::SPINNER_STROKE),
                palette.pending,
            );
        }
    }
    painter.rect_stroke(rect, 0.0, Stroke::new(BORDER, palette.border), StrokeKind::Inside);
}

/// The colour of a `Chip`. Colour never carries meaning alone: the text says it too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipColor {
    Neutral,
    Success,
    Warning,
}

impl ChipColor {
    fn tone(self, palette: &Palette) -> Option<Color32> {
        match self {
            ChipColor::Neutral => None,
            ChipColor::Success => Some(palette.success),
            ChipColor::Warning => Some(palette.warning),
        }
    }
}

/// A `Chip`: a static tag, or an action chip when `clickable`.
pub fn chip(ui: &mut Ui, label: &str, color: ChipColor, clickable: bool, palette: &Palette) -> Response {
    let tone = color.tone(palette);
    let ink = tone.unwrap_or(palette.text_primary);
    let words = galley(ui, text(label, Type::BodySmall, ink));
    let size = vec2(
        (words.size().x + 2.0 * du(dim::GUTTER)).ceil(),
        du(dim::ROW_DENSE),
    );
    let sense = if clickable { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let role = if clickable {
        egui::Role::Button
    } else {
        egui::Role::Label
    };
    response.widget_info(|| WidgetInfo::labeled(role, true, label));
    if ui.is_rect_visible(rect) {
        let fills = match tone {
            Some(tone) => Fills {
                rest: mix(palette.surface, tone, 0.14),
                hover: mix(palette.surface, tone, 0.14),
                pressed: mix(palette.pressed, tone, 0.22),
            },
            None => Fills::plain(palette),
        };
        let state = if clickable {
            paint_body(ui, rect, &response, fills, &Look::IDLE, true, palette)
        } else {
            ui.painter().rect_filled(rect, 0.0, fills.rest);
            ControlState::Idle
        };
        paint_galley(ui, rect, words, ink);
        let mut outline = Outline::of(state, palette);
        if let Some(tone) = tone
            && state == ControlState::Idle
        {
            outline.colour = mix(palette.surface, tone, 0.55);
        }
        paint_border(ui, rect, outline, &Look::IDLE, palette);
        paint_focus_ring(ui, &response, rect, palette);
    }
    if clickable {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// `ChipSet`: chips in wrapping rows, `1 DU` apart.
pub fn chip_set<R>(ui: &mut Ui, chips: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(du(dim::PAD_TIGHT), du(dim::PAD_TIGHT));
        ui.spacing_mut().interact_size.y = du(dim::ROW_DENSE);
        chips(ui)
    })
    .inner
}

/// One row of a `PropertyList` (draft).
pub struct Property<'a> {
    pub label: &'a str,
    pub value: String,
    /// The accessible name of the copy button, when the value can be copied.
    pub copy: Option<&'a str>,
}

/// `PropertyList` (draft): labels and values; the label goes above the value below `60 DU`.
pub fn property_list(ui: &mut Ui, id: Id, properties: &[Property<'_>], palette: &Palette) {
    let width = ui.available_width();
    let stacked = width < du(60.0);
    let gap = du(dim::GUTTER);
    let pad = du(dim::PAD_TIGHT);
    let copy_side = du(6.0);
    for (index, property) in properties.iter().enumerate() {
        let (label_width, value_width) = if stacked {
            (width, width)
        } else {
            ((width - gap) * 0.4, (width - gap) * 0.6)
        };
        let room = value_width - property.copy.map_or(0.0, |_| copy_side + pad);
        let label_style = if stacked { Type::Caption } else { Type::Body };
        let label = wrapped(
            ui,
            text(property.label, label_style, palette.text_secondary),
            label_width,
        );
        let value = wrapped(ui, text(&property.value, Type::Body, palette.text_primary), room);
        let content = if stacked {
            label.size().y
                + value
                    .size()
                    .y
                    .max(copy_side * f32::from(u8::from(property.copy.is_some())))
        } else {
            label.size().y.max(value.size().y).max(copy_side)
        };
        let height = (content + 2.0 * pad).max(du(dim::ROW_STANDARD));
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let (label_rect, value_rect) = if stacked {
            let label_rect = Rect::from_min_size(rect.min + vec2(0.0, pad), vec2(width, label.size().y));
            let value_rect =
                Rect::from_min_max(pos2(rect.left(), label_rect.bottom()), rect.max - vec2(0.0, pad));
            (label_rect, value_rect)
        } else {
            let inner = rect.shrink2(vec2(0.0, pad));
            (
                Rect::from_min_size(inner.min, vec2(label_width, inner.height())),
                Rect::from_min_max(pos2(inner.left() + label_width + gap, inner.top()), inner.max),
            )
        };
        let label_size = label.size();
        let label_at = pos2(label_rect.left(), label_rect.center().y - label_size.y / 2.0);
        let label_response = ui.interact(
            Rect::from_min_size(label_at.round(), label_size),
            id.with(("label", index)),
            selectable_sense(),
        );
        label_response.widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, property.label));
        paint_selectable(
            ui,
            &label_response,
            label_at.round(),
            label,
            palette.text_secondary,
        );
        let value_size = value.size();
        let value_at = pos2(value_rect.left(), value_rect.center().y - value_size.y / 2.0);
        let value_response = ui.interact(
            Rect::from_min_size(value_at.round(), value_size),
            id.with(("value", index)),
            selectable_sense(),
        );
        value_response.widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, &property.value));
        paint_selectable(ui, &value_response, value_at.round(), value, palette.text_primary);
        if let Some(name) = property.copy {
            let button = Rect::from_min_size(
                pos2(
                    value_rect.right() - copy_side,
                    value_rect.center().y - copy_side / 2.0,
                ),
                vec2(copy_side, copy_side),
            );
            copy_button(
                ui,
                button,
                id.with(("copy", index)),
                name,
                &property.value,
                palette,
            );
        }
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - BORDER / 2.0,
            Stroke::new(BORDER, palette.divider),
        );
    }
}

/// A `6 DU` copy button that flashes its result, as in `PropertyList` and `CodeBlock`.
fn copy_button(ui: &mut Ui, rect: Rect, key: Id, name: &str, value: &str, palette: &Palette) {
    let mut activity = Activity::load(ui.ctx(), key);
    let now = ui.input(|input| input.time);
    let look = activity.look(now, result_hold());
    let clicked = ui
        .scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            IconButton::new(icon::CONTENT_COPY, name)
                .size(6.0)
                .icon_size(3.5)
                .look(look)
                .show(ui, palette)
                .clicked()
        })
        .inner;
    if clicked {
        ui.ctx().copy_text(value.to_owned());
        activity.start(now);
        activity.finish(Outcome::Success, now);
    }
    activity.store(ui.ctx(), key);
}

/// `AsyncCheckbox` without a handler: the value changes on click. Returns whether it changed.
pub fn checkbox(ui: &mut Ui, checked: &mut bool, label: &str, palette: &Palette) -> bool {
    let words = galley(ui, weighted(label, Type::Body, MEDIUM, palette.text_primary));
    let side = du(5.0);
    let gap = du(dim::GUTTER);
    let pad = vec2(du(dim::PAD_TIGHT), du(dim::PAD_SMALL));
    let size = vec2(side + gap + words.size().x, side.max(words.size().y)) + 2.0 * pad;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let value = *checked;
    response.widget_info(|| WidgetInfo::selected(egui::Role::CheckBox, true, value, label));
    let changed = response.clicked();
    if changed {
        *checked = !*checked;
    }
    let inner = rect.shrink2(pad);
    let boxed = Rect::from_min_size(
        pos2(inner.left(), inner.center().y - side / 2.0),
        vec2(side, side),
    );
    let hover = hover_amount(ui, &response);
    let (fill, outline) = if *checked {
        let fill = tint(palette.accent, 0.15 + 0.07 * hover);
        (fill, Outline::of(ControlState::Selected, palette))
    } else {
        let fill = mix(palette.surface, palette.hover, hover);
        let state = if hover > 0.5 {
            ControlState::Pressed
        } else {
            ControlState::Idle
        };
        (fill, Outline::of(state, palette))
    };
    ui.painter().rect_filled(boxed, 0.0, fill);
    if *checked {
        paint_icon(ui, icon::CHECK, boxed.center(), du(3.5), palette.accent);
    }
    paint_border(ui, boxed, outline, &Look::IDLE, palette);
    let text_rect = Rect::from_min_max(pos2(boxed.right() + gap, inner.top()), inner.max);
    super::text::paint_aligned(ui, text_rect, words, egui::Align::Min, palette.text_primary);
    paint_focus_ring(ui, &response, rect, palette);
    let _ = response.on_hover_cursor(CursorIcon::PointingHand);
    changed
}

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

/// The two panes of a `SplitPane`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Start,
    End,
}

/// `SplitPane`, side by side: two panes and a divider that sets the share of the first.
pub struct SplitPane {
    pub id: Id,
    pub initial: f32,
    /// Minimum pane size in DU.
    pub min_size: f32,
}

impl SplitPane {
    pub fn show(&self, ui: &mut Ui, ratio: &mut f32, palette: &Palette, mut pane: impl FnMut(&mut Ui, Pane)) {
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, Sense::hover());
        let minimum = du(self.min_size).min(rect.width() / 2.0);
        let clamp = |share: f32| {
            let lowest = minimum / rect.width().max(1.0);
            share.clamp(lowest, 1.0 - lowest)
        };
        *ratio = clamp(*ratio);
        let hit = du(dim::GUTTER);
        let x = rect.left() + rect.width() * *ratio;
        let handle = Rect::from_x_y_ranges(x - hit / 2.0..=x + hit / 2.0, rect.y_range());
        let response = ui.interact(handle, self.id.with("divider"), Sense::click_and_drag());
        let percent = (*ratio * 100.0).round();
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Splitter, true, format!("{percent}%")));
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            *ratio = clamp((pointer.x - rect.left()) / rect.width());
        }
        if response.double_clicked() {
            *ratio = clamp(self.initial);
        }
        if response.has_focus() {
            ui.input(|input| {
                let step = if input.modifiers.shift { 0.1 } else { 0.02 };
                if input.key_pressed(Key::ArrowLeft) {
                    *ratio = clamp(*ratio - step);
                }
                if input.key_pressed(Key::ArrowRight) {
                    *ratio = clamp(*ratio + step);
                }
                if input.key_pressed(Key::Home) {
                    *ratio = clamp(0.0);
                }
                if input.key_pressed(Key::End) {
                    *ratio = clamp(1.0);
                }
                if input.key_pressed(Key::Enter) {
                    *ratio = clamp(self.initial);
                }
            });
        }
        let x = (rect.left() + rect.width() * *ratio).round();
        let first = Rect::from_min_max(rect.min, pos2(x, rect.bottom()));
        let second = Rect::from_min_max(pos2(x, rect.top()), rect.max);
        ui.scope_builder(UiBuilder::new().max_rect(first), |ui| {
            ui.set_clip_rect(first);
            pane(ui, Pane::Start);
        });
        ui.scope_builder(UiBuilder::new().max_rect(second), |ui| {
            ui.set_clip_rect(second);
            pane(ui, Pane::End);
        });
        let lit = response.hovered() || response.dragged() || response.has_focus();
        let (width, colour) = if lit {
            (BORDER_THICK, palette.accent)
        } else {
            (BORDER, palette.border)
        };
        let line = Rect::from_x_y_ranges(x - width / 2.0..=x + width / 2.0, rect.y_range());
        ui.painter().rect_filled(line, 0.0, colour);
        let grip = Rect::from_center_size(
            pos2(x, rect.center().y),
            vec2(du(dim::PAD_TIGHT), du(dim::ICON_LARGE)),
        );
        let mut y = grip.top();
        while y < grip.bottom() {
            let stripe = Rect::from_min_size(pos2(grip.left(), y), vec2(grip.width(), BORDER));
            ui.painter().rect_filled(stripe, 0.0, palette.border_strong);
            y += du(dim::GAP_MICRO);
        }
        if lit {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_avatar_colour_depends_only_on_the_id() {
        assert_eq!(avatar_colour("firefox", true), avatar_colour("firefox", true));
        assert_ne!(avatar_colour("firefox", true), avatar_colour("firefox", false));
    }

    #[test]
    fn hsv_matches_css_for_a_known_colour() {
        assert_eq!(hsv(0.0, 1.0, 1.0), Color32::from_rgb(255, 0, 0));
        assert_eq!(hsv(210.0, 0.5, 0.8), Color32::from_rgb(102, 153, 204));
    }

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
