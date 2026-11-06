//! Operator UI controls that egui has no counterpart for.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

use eframe::egui::{
    self, Align2, Color32, ColorImage, FontId, FontSelection, Galley, Id, Rect, Response, RichText, Sense,
    Stroke, StrokeKind, TextEdit, TextWrapMode, TextureHandle, TextureOptions, Ui, Vec2, WidgetInfo,
    WidgetText, pos2, vec2,
};

use super::glyph;
use super::theme::{
    BORDER, BORDER_THICK, COMPACT_ROW, GUTTER, MARGIN_SCREEN, PAD_CARD, PAD_TIGHT, Palette, STANDARD_ROW,
    Type, Weight, du, styled, tint, weighted,
};
use crate::i18n::Text;
use crate::icons::{Decoded, decode};

/// Colour transitions between states, in seconds.
const TRANSITION: f32 = 0.15;
/// How long a local result state stays on screen, in seconds.
const COPY_FLASH: f64 = 0.8;
pub const SEGMENT_HEIGHT: f32 = du(9.0);
const BADGE_HEIGHT: f32 = du(5.0);
const ICON_SIZE: f32 = du(4.0);

/// `ControlState` of the design system, as far as a border shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlState {
    Idle,
    Pending,
    Success,
    Error,
}

/// A point on the outline of a rectangle, clockwise from the top left corner.
fn point_on_outline(rect: Rect, distance: f32) -> egui::Pos2 {
    let (width, height) = (rect.width(), rect.height());
    let distance = distance.rem_euclid(2.0 * (width + height));
    if distance < width {
        pos2(rect.left() + distance, rect.top())
    } else if distance < width + height {
        pos2(rect.right(), rect.top() + distance - width)
    } else if distance < 2.0 * width + height {
        pos2(rect.right() - (distance - width - height), rect.bottom())
    } else {
        pos2(rect.left(), rect.bottom() - (distance - 2.0 * width - height))
    }
}

/// The state border. The stroke stays inside the rectangle, so a state never changes the size.
pub fn state_border(ui: &Ui, rect: Rect, state: ControlState, palette: &Palette) {
    let painter = ui.painter();
    let time = ui.input(|input| input.time) as f32;
    match state {
        ControlState::Idle => {
            painter.rect_stroke(rect, 0.0, Stroke::new(BORDER, palette.border), StrokeKind::Inside);
        }
        ControlState::Pending => {
            painter.rect_stroke(rect, 0.0, Stroke::new(BORDER, palette.border), StrokeKind::Inside);
            let inset = rect.shrink(BORDER * 0.75);
            let perimeter = 2.0 * (inset.width() + inset.height());
            let start = (time * 0.6).fract() * perimeter;
            let points: Vec<_> = (0..=48)
                .map(|step| point_on_outline(inset, start + perimeter * 0.25 * step as f32 / 48.0))
                .collect();
            let stroke = Stroke::new(BORDER * 1.5, palette.pending);
            painter.add(egui::Shape::line(points, stroke));
            ui.ctx().request_repaint();
        }
        ControlState::Success | ControlState::Error => {
            let colour = if state == ControlState::Success {
                palette.success
            } else {
                palette.error
            };
            let wave = ((time / 0.8 * std::f32::consts::TAU).sin() + 1.0) / 2.0;
            let stroke = Stroke::new(BORDER_THICK, tint(colour, 0.55 + 0.45 * wave));
            painter.rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
            ui.ctx().request_repaint();
        }
    }
}

/// A row of the side bar: a glyph, a label, a count on the right.
pub fn nav_row(
    ui: &mut Ui,
    glyph: &str,
    label: &str,
    count: usize,
    selected: bool,
    palette: &Palette,
) -> Response {
    let size = vec2(ui.available_width(), COMPACT_ROW);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let text = format!("{label} {count}");
    response.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, selected, text.clone()));
    let painter = ui.painter();
    let foreground = if selected {
        palette.accent
    } else if count == 0 {
        palette.text_disabled
    } else {
        palette.text_primary
    };
    if selected {
        painter.rect_filled(rect, 0.0, tint(palette.accent, 0.15));
        let marker = Rect::from_min_size(rect.min, vec2(BORDER_THICK, rect.height()));
        painter.rect_filled(marker, 0.0, palette.accent);
    } else {
        painter.rect_filled(rect, 0.0, tint(palette.surface, hover_amount(ui, &response)));
    }
    let font = FontId::monospace(Type::Body.size());
    let middle = rect.center().y;
    painter.text(
        pos2(rect.left() + GUTTER, middle),
        Align2::LEFT_CENTER,
        glyph,
        FontId::monospace(16.0),
        foreground,
    );
    painter.text(
        pos2(rect.left() + du(7.0), middle),
        Align2::LEFT_CENTER,
        label,
        font,
        foreground,
    );
    let number = FontId::monospace(Type::Subtitle.size());
    painter.text(
        pos2(rect.right() - GUTTER, middle),
        Align2::RIGHT_CENTER,
        count.to_string(),
        number,
        palette.text_secondary,
    );
    pointing(response, true)
}

/// The heading of a group in the side bar.
pub fn nav_heading(ui: &mut Ui, text: &str, palette: &Palette) {
    let size = vec2(ui.available_width(), du(8.0));
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let galley = galley_of(
        ui,
        weighted(text, Type::Subtitle, Weight::Bold, palette.text_secondary),
    );
    let top = rect.bottom() - galley.size().y - PAD_TIGHT;
    ui.painter()
        .galley(pos2(rect.left() + GUTTER, top), galley, palette.text_secondary);
}

fn galley_of(ui: &Ui, text: RichText) -> Arc<Galley> {
    WidgetText::from(text).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        FontSelection::Default,
    )
}

/// Text laid out by itself, centred in a rectangle on whole pixels.
fn galley_centred(ui: &Ui, rect: Rect, galley: Arc<Galley>, colour: Color32) {
    let corner = (rect.center() - galley.size() / 2.0).round();
    ui.painter().galley(corner, galley, colour);
}

/// One choice out of several: a sunken bar with a tinted block sliding to the choice.
/// Returns the index clicked.
pub fn segmented(ui: &mut Ui, labels: &[&str], selected: usize, palette: &Palette) -> Option<usize> {
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| {
            let text = weighted(*label, Type::Subtitle, Weight::Bold, palette.accent);
            (galley_of(ui, text).size().x + 2.0 * PAD_CARD).ceil()
        })
        .collect();
    let size = vec2(widths.iter().sum(), SEGMENT_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let bar_id = ui.scope_id().with(labels.first().copied().unwrap_or_default());
    ui.painter().rect_filled(rect, 0.0, palette.surface_alt);
    let outline = Stroke::new(BORDER, palette.border);
    ui.painter().rect_stroke(rect, 0.0, outline, StrokeKind::Inside);

    let mut clicked = None;
    let mut lit = Vec::with_capacity(labels.len());
    let mut left = rect.left();
    let mut cells = Vec::with_capacity(labels.len());
    for (index, width) in widths.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(left, rect.top()), vec2(*width, rect.height()));
        left += width;
        cells.push(cell);
        let response = ui.interact(cell, bar_id.with(index), Sense::click());
        let label = labels[index];
        let chosen = index == selected;
        response.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, chosen, label));
        let hover = hover_amount(ui, &response);
        lit.push(hover);
        if !chosen {
            let inner = cell.shrink(BORDER);
            ui.painter()
                .rect_filled(inner, 0.0, tint(palette.border, 0.35 * hover));
        }
        let response = pointing(response, !chosen);
        if response.clicked() {
            clicked = Some(index);
        }
    }

    if let Some(target) = cells.get(selected) {
        let ctx = ui.ctx();
        let start = ctx.animate_value_with_time(bar_id.with("left"), target.left(), 0.15);
        let end = ctx.animate_value_with_time(bar_id.with("right"), target.right(), 0.15);
        let block = Rect::from_x_y_ranges(start..=end, rect.y_range());
        ui.painter().rect_filled(block, 0.0, tint(palette.accent, 0.15));
        let stroke = Stroke::new(BORDER, palette.accent);
        ui.painter().rect_stroke(block, 0.0, stroke, StrokeKind::Inside);
    }

    for (index, (label, cell)) in labels.iter().zip(&cells).enumerate() {
        let (weight, colour) = if index == selected {
            (Weight::Bold, palette.accent)
        } else {
            (
                Weight::Medium,
                mix(palette.text_secondary, palette.text_primary, lit[index]),
            )
        };
        let galley = galley_of(ui, weighted(*label, Type::Subtitle, weight, colour));
        galley_centred(ui, *cell, galley, colour);
    }
    clicked
}

/// A square button holding one glyph.
pub fn icon_button(ui: &mut Ui, glyph: &str, side: f32, hint: &str, palette: &Palette) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, format!("{glyph} {hint}")));
    let colour = paint_button(ui, rect, &response, Tone::Plain, palette);
    let font = FontId::monospace(ICON_SIZE);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, glyph, font, colour);
    pointing(response, true).on_hover_text(hint)
}

/// The search field: the glyph and the text in one line, a clearing button flush with the right
/// edge. Returns whether the text changed.
pub fn search_field(ui: &mut Ui, query: &mut String, width: f32, text: &Text, palette: &Palette) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(width, STANDARD_ROW), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.surface);
    let glyph_at = pos2(rect.left() + PAD_CARD + ICON_SIZE / 2.0, rect.center().y);
    let glyph_font = FontId::monospace(ICON_SIZE);
    let painter = ui.painter();
    painter.text(
        glyph_at,
        Align2::CENTER_CENTER,
        glyph::MAGNIFYING_GLASS,
        glyph_font,
        palette.text_secondary,
    );

    let clear_width = if query.is_empty() { 0.0 } else { STANDARD_ROW };
    let text_rect = Rect::from_min_max(
        pos2(rect.left() + PAD_CARD + ICON_SIZE + GUTTER, rect.top()),
        pos2(rect.right() - clear_width - GUTTER, rect.bottom()),
    );
    let edit = TextEdit::singleline(query)
        .hint_text(styled(text.search, Type::Body, palette.text_secondary))
        .frame(egui::Frame::NONE)
        .margin(egui::Margin::ZERO)
        .desired_width(text_rect.width())
        .min_size(text_rect.size())
        .align(Align2::LEFT_CENTER);
    let response = ui.put(text_rect, edit);
    let mut changed = response.changed();

    if clear_width > 0.0 {
        let cell = Rect::from_min_max(pos2(rect.right() - clear_width, rect.top()), rect.max);
        let clear = ui.interact(cell, ui.scope_id().with("clear search"), Sense::click());
        clear.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, text.clear_search));
        let colour = mix(
            palette.text_secondary,
            palette.text_primary,
            hover_amount(ui, &clear),
        );
        let font = FontId::monospace(ICON_SIZE);
        ui.painter()
            .text(cell.center(), Align2::CENTER_CENTER, glyph::X, font, colour);
        if pointing(clear, true).clicked() {
            query.clear();
            changed = true;
            response.request_focus();
        }
    }
    let border = if response.has_focus() {
        palette.border_strong
    } else {
        palette.border
    };
    ui.painter()
        .rect_stroke(rect, 0.0, Stroke::new(BORDER, border), StrokeKind::Inside);
    changed
}

/// How far the pointer state has progressed, 0 to 1, over the 150 ms of the design system.
pub fn hover_amount(ui: &Ui, response: &Response) -> f32 {
    let id = response.id.with("hover");
    ui.ctx()
        .animate_bool_with_time(id, response.hovered() && response.enabled(), TRANSITION)
}

fn press_amount(ui: &Ui, response: &Response) -> f32 {
    let id = response.id.with("press");
    ui.ctx()
        .animate_bool_with_time(id, response.is_pointer_button_down_on(), TRANSITION / 2.0)
}

fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    from.lerp_to_gamma(to, amount)
}

/// Colours of a button at rest, under the pointer and pressed.
struct Shades {
    rest: Color32,
    hover: Color32,
    press: Color32,
    border: Color32,
    border_active: Color32,
    text: Color32,
}

fn shades(tone: Tone, palette: &Palette) -> Shades {
    match tone {
        Tone::Plain => Shades {
            rest: palette.surface,
            hover: mix(palette.surface, palette.border, 0.35),
            press: mix(palette.surface, palette.border, 0.65),
            border: palette.border,
            border_active: palette.border_strong,
            text: palette.text_primary,
        },
        Tone::Danger => Shades {
            rest: tint(palette.error, 0.15),
            hover: tint(palette.error, 0.24),
            press: tint(palette.error, 0.32),
            border: tint(palette.error, 0.7),
            border_active: palette.error,
            text: palette.error,
        },
        Tone::Success => Shades {
            rest: tint(palette.success, 0.15),
            hover: tint(palette.success, 0.15),
            press: tint(palette.success, 0.15),
            border: palette.success,
            border_active: palette.success,
            text: palette.success,
        },
    }
}

/// Paints the body of a button and returns the colour for what goes on it.
fn paint_button(ui: &Ui, rect: Rect, response: &Response, tone: Tone, palette: &Palette) -> Color32 {
    if !response.enabled() {
        ui.painter().rect_filled(rect, 0.0, palette.disabled_surface);
        let stroke = Stroke::new(BORDER, palette.disabled_border);
        ui.painter().rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
        return palette.text_disabled;
    }
    let colours = shades(tone, palette);
    let hover = hover_amount(ui, response);
    let press = press_amount(ui, response);
    let fill = mix(mix(colours.rest, colours.hover, hover), colours.press, press);
    let border = mix(colours.border, colours.border_active, hover.max(press));
    ui.painter().rect_filled(rect, 0.0, fill);
    ui.painter()
        .rect_stroke(rect, 0.0, Stroke::new(BORDER, border), StrokeKind::Inside);
    colours.text
}

/// An action button, sized by its text. The dangerous style is the red one of the specification.
pub fn action_button(
    ui: &mut Ui,
    label: &str,
    glyph: Option<&str>,
    tone: Tone,
    enabled: bool,
    palette: &Palette,
) -> Response {
    let text = glyph.map_or_else(|| label.to_owned(), |glyph| format!("{glyph} {label}"));
    let measured = galley_of(
        ui,
        weighted(text.as_str(), Type::Body, Weight::Medium, palette.text_primary),
    );
    let size = vec2((measured.size().x + 2.0 * MARGIN_SCREEN).ceil(), STANDARD_ROW);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, mut response) = ui.allocate_exact_size(size, sense);
    response.flags.set(egui::response::Flags::ENABLED, enabled);
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, enabled, text.clone()));
    let colour = paint_button(ui, rect, &response, tone, palette);
    let galley = galley_of(ui, weighted(text, Type::Body, Weight::Medium, colour));
    galley_centred(ui, rect, galley, colour);
    pointing(response, enabled)
}

fn pointing(response: Response, enabled: bool) -> Response {
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// A button that copies text and confirms it: a check mark in green for the 800 ms of the local
/// result state. The label stays, so the button does not change its width.
pub fn copy_button(ui: &mut Ui, label: &str, text: &str, palette: &Palette) -> Response {
    let key = ui.scope_id().with(("copied", label));
    let now = ui.input(|input| input.time);
    let copied_at: Option<f64> = ui.ctx().data(|data| data.get_temp(key));
    let flashing = copied_at.is_some_and(|at| now - at < COPY_FLASH);
    let (glyph, tone) = if flashing {
        (glyph::CHECK, Tone::Success)
    } else {
        (glyph::COPY, Tone::Plain)
    };
    let response = action_button(ui, label, Some(glyph), tone, true, palette);
    if response.clicked() {
        ui.ctx().copy_text(text.to_owned());
        ui.ctx().data_mut(|data| data.insert_temp(key, now));
    }
    if flashing || response.clicked() {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    response
}

/// A small link to another package.
pub fn chip(ui: &mut Ui, text: &str, palette: &Palette) -> Response {
    let measured = galley_of(ui, styled(text, Type::Subtitle, palette.accent));
    let size = vec2((measured.size().x + 2.0 * GUTTER).ceil(), du(6.0));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, text));
    paint_button(ui, rect, &response, Tone::Plain, palette);
    galley_centred(ui, rect, measured, palette.accent);
    pointing(response, true)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Danger,
    Success,
}

/// A small outlined label: the source of a program, a warning next to its name.
pub fn badge(ui: &mut Ui, text: &str, colour: Color32) {
    let galley = galley_of(ui, styled(text, Type::Subtitle, colour));
    let size = vec2((galley.size().x + 2.0 * PAD_TIGHT).ceil(), BADGE_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_stroke(rect, 0.0, Stroke::new(BORDER, colour), StrokeKind::Inside);
    galley_centred(ui, rect, galley, colour);
}

enum Slot {
    Waiting,
    Ready(TextureHandle),
    Missing,
}

struct Answer {
    path: PathBuf,
    image: Option<Decoded>,
}

/// Decodes icons on one thread, and only those the list has actually scrolled to.
pub struct IconCache {
    slots: HashMap<PathBuf, Slot>,
    requests: Sender<PathBuf>,
    answers: Receiver<Answer>,
}

impl IconCache {
    pub fn new(ctx: &egui::Context) -> Self {
        let (requests, incoming) = channel::<PathBuf>();
        let (reply, answers) = channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            for path in incoming {
                let image = decode(&path);
                if reply.send(Answer { path, image }).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
        Self {
            slots: HashMap::new(),
            requests,
            answers,
        }
    }

    /// Takes what the decoder finished. Called once a frame.
    pub fn receive(&mut self, ctx: &egui::Context) {
        while let Ok(answer) = self.answers.try_recv() {
            let slot = answer.image.map_or(Slot::Missing, |image| {
                let picture = ColorImage::from_rgba_premultiplied(image.size, &image.rgba);
                let name = answer.path.display().to_string();
                Slot::Ready(ctx.load_texture(name, picture, TextureOptions::LINEAR))
            });
            self.slots.insert(answer.path, slot);
        }
    }

    fn texture(&mut self, path: &Path) -> Option<&TextureHandle> {
        if !self.slots.contains_key(path) {
            let asked = self.requests.send(path.to_path_buf()).is_ok();
            let slot = if asked { Slot::Waiting } else { Slot::Missing };
            self.slots.insert(path.to_path_buf(), slot);
        }
        match self.slots.get(path) {
            Some(Slot::Ready(texture)) => Some(texture),
            _ => None,
        }
    }
}

/// Soft background of a placeholder, always the same for the same program.
fn placeholder_colour(key: &str, palette: &Palette) -> Color32 {
    let hash = key
        .bytes()
        .fold(5381_u32, |hash, byte| hash.wrapping_mul(33) ^ u32::from(byte));
    let channel = |shift: u32| 40 + ((hash >> shift) & 0x5F) as u8;
    let colour = Color32::from_rgb(channel(0), channel(8), channel(16));
    if palette.background.r() > 128 {
        tint(colour, 0.5)
    } else {
        colour
    }
}

/// The icon of a program, or its initial on a soft square when it has none.
pub fn avatar(
    ui: &Ui,
    rect: Rect,
    icon: Option<&Path>,
    title: &str,
    cache: &mut IconCache,
    palette: &Palette,
) {
    if let Some(texture) = icon.and_then(|path| cache.texture(path)) {
        let size: Vec2 = texture.size_vec2();
        let scale = (rect.width() / size.x).min(rect.height() / size.y);
        let target = Rect::from_center_size(rect.center(), size * scale);
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter().image(texture.id(), target, uv, Color32::WHITE);
        return;
    }
    ui.painter()
        .rect_filled(rect, 0.0, placeholder_colour(title, palette));
    let initial: String = title
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    let font = FontId::monospace(rect.height() * 0.5);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        font,
        palette.text_primary,
    );
}

/// A stable id for a widget of the list.
pub fn row_id(key: &str) -> Id {
    Id::unique(("row", key))
}
