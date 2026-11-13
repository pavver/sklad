//! Content components: `Avatar`, `Chip`, `PropertyList` (draft) and `AsyncCheckbox`
//! (synchronous use).

use eframe::egui::{
    self, Color32, CursorIcon, Id, Rect, Response, Sense, Stroke, StrokeKind, TextureHandle, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};

use super::button::{Fills, IconButton, paint_body};
use super::icon::{self, paint_icon};
use super::state::{
    Activity, BORDER, ControlState, Look, Outcome, Outline, hover_amount, paint_border, paint_focus_ring,
    paint_spinner, result_hold,
};
use super::text::{galley, paint_galley, paint_selectable, selectable_sense, wrapped};
use super::tokens::{BOLD, MEDIUM, Palette, Type, dim, du, mix, text, tint, weighted};

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

/// An input chip: a neutral chip with a remove button `6 DU` wide at its end, which turns
/// `error` on hover. Returns whether the button was pressed.
pub fn input_chip(ui: &mut Ui, id: Id, label: &str, remove_label: &str, palette: &Palette) -> bool {
    let words = galley(ui, text(label, Type::BodySmall, palette.text_primary));
    let height = du(dim::ROW_DENSE);
    let size = vec2((words.size().x + du(dim::GUTTER) + height).ceil(), height);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let button = Rect::from_min_max(pos2(rect.right() - height, rect.top()), rect.max);
    let response = ui.interact(button, id, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, remove_label));
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 0.0, palette.surface);
        let hover = hover_amount(ui, &response);
        let press = super::state::press_amount(ui, &response);
        ui.painter().rect_filled(
            button,
            0.0,
            mix(tint(palette.hover, hover), palette.pressed, press),
        );
        let words_rect = Rect::from_min_max(
            pos2(rect.left() + du(dim::GUTTER), rect.top()),
            pos2(button.left(), rect.bottom()),
        );
        super::text::paint_aligned(ui, words_rect, words, egui::Align::Min, palette.text_primary);
        let ink = mix(palette.icon, palette.error, hover);
        paint_icon(ui, icon::CLOSE, button.center(), du(3.5), ink);
        let state = if response.is_pointer_button_down_on() {
            ControlState::Pressed
        } else {
            ControlState::Idle
        };
        paint_border(ui, rect, Outline::of(state, palette), &Look::IDLE, palette);
        paint_focus_ring(ui, &response, button, palette);
    }
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(remove_label);
    response.clicked()
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
}
