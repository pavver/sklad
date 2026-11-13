//! Bars and rows: `Header`, `Toolbar` (draft), `Counter`, `Divider`, section headings.

use eframe::egui::{self, Align, Layout, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use super::state::BORDER;
use super::text::{galley, paint_aligned, paint_galley, truncated};
use super::tokens::{BOLD, Palette, Type, dim, du, text, weighted};

/// Lays out `end` from the right edge of `rect` and returns the rectangle it took.
fn from_the_end(ui: &mut Ui, rect: Rect, gap: f32, end: impl FnOnce(&mut Ui)) -> Rect {
    let layout = Layout::right_to_left(Align::Center);
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(layout), |ui| {
        ui.spacing_mut().item_spacing = vec2(gap, 0.0);
        end(ui);
    })
    .response
    .rect
}

/// `Header`: the top bar with a title, a subtitle and actions.
pub fn header(
    ui: &mut Ui,
    title: &str,
    subtitle: Option<&str>,
    palette: &Palette,
    actions: impl FnOnce(&mut Ui),
) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), du(13.5)), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.surface);
    let inner = rect.shrink2(vec2(du(dim::MARGIN_SCREEN), 0.0));
    let taken = from_the_end(ui, inner, du(dim::GUTTER), actions);
    let width = (taken.left() - inner.left() - du(dim::GUTTER)).max(0.0);
    let kind = if subtitle.is_some() {
        Type::RowTitle
    } else {
        Type::AppTitle
    };
    let title = truncated(ui, weighted(title, kind, BOLD, palette.text_primary), width);
    match subtitle {
        Some(subtitle) => {
            let subtitle = truncated(ui, text(subtitle, Type::Caption, palette.text_secondary), width);
            let total = title.size().y + subtitle.size().y;
            let top = inner.center().y - total / 2.0;
            let title_height = title.size().y;
            ui.painter()
                .galley(pos2(inner.left(), top).round(), title, palette.text_primary);
            ui.painter().galley(
                pos2(inner.left(), top + title_height).round(),
                subtitle,
                palette.text_secondary,
            );
        }
        None => paint_aligned(ui, inner, title, Align::Min, palette.text_primary),
    }
    line_below(ui, rect, palette.border);
}

fn line_below(ui: &Ui, rect: Rect, colour: egui::Color32) {
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - BORDER / 2.0,
        Stroke::new(BORDER, colour),
    );
}

/// A heading inside a screen in `section_title` style. The design system has the style but no
/// component for it, so this is only text.
pub fn section_title(ui: &mut Ui, content: &str, palette: &Palette) {
    ui.label(text(content, Type::SectionTitle, palette.text_primary));
}

/// `Toolbar` (draft): a title, commands as icon buttons, and end controls that never overflow.
///
/// The draft specification is still changing; the overflow menu is not ported, because the bar of
/// sklad holds one command.
pub fn toolbar(
    ui: &mut Ui,
    title: &str,
    palette: &Palette,
    commands: impl FnOnce(&mut Ui),
    end: impl FnOnce(&mut Ui),
) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), du(dim::ROW_STANDARD)), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.surface);
    let inner = rect.shrink2(vec2(du(dim::PAD_TIGHT), 0.0));
    let gap = du(dim::PAD_TIGHT);
    let taken = from_the_end(ui, inner, gap, end);

    let title_width = (taken.left() - inner.left()).max(0.0) * 0.5;
    let title = truncated(
        ui,
        weighted(title, Type::Body, 500, palette.text_primary),
        title_width,
    );
    let title_rect = Rect::from_min_size(
        inner.min + vec2(du(dim::PAD_TIGHT), 0.0),
        vec2(title.size().x, inner.height()),
    );
    paint_aligned(ui, title_rect, title, Align::Min, palette.text_primary);
    let separator_x = title_rect.right() + du(dim::PAD_TIGHT) * 2.0;
    let separator = Rect::from_center_size(
        pos2(separator_x + BORDER / 2.0, inner.center().y),
        vec2(BORDER, du(6.0)),
    );
    ui.painter().rect_filled(separator, 0.0, palette.divider);
    let commands_rect = Rect::from_min_max(
        pos2(separator.right() + du(dim::PAD_TIGHT), inner.top()),
        pos2(taken.left() - gap, inner.bottom()),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(commands_rect)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 0.0);
            commands(ui);
        },
    );
    line_below(ui, rect, palette.border);
}

/// `Counter` tones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Neutral,
}

/// `Counter`: a compact number. Above `max` it shows `max+`; at zero it shows only with
/// `show_zero`.
pub fn counter_text(count: usize, max: usize) -> String {
    if count > max {
        format!("{max}+")
    } else {
        count.to_string()
    }
}

/// The size a counter takes.
pub fn counter_size(ui: &Ui, count: usize, max: usize) -> egui::Vec2 {
    let label = galley(
        ui,
        weighted(
            counter_text(count, max),
            Type::Caption,
            BOLD,
            egui::Color32::WHITE,
        ),
    );
    let height = du(4.5);
    vec2(
        (label.size().x + 2.0 * du(dim::PAD_TIGHT)).max(height).ceil(),
        height,
    )
}

pub fn paint_counter(ui: &Ui, rect: Rect, count: usize, max: usize, tone: Tone, palette: &Palette) {
    match tone {
        Tone::Neutral => {
            ui.painter().rect_filled(rect, 0.0, palette.surface_alt);
            ui.painter().rect_stroke(
                rect,
                0.0,
                Stroke::new(BORDER, palette.border),
                egui::StrokeKind::Inside,
            );
            let label = galley(
                ui,
                weighted(
                    counter_text(count, max),
                    Type::Caption,
                    BOLD,
                    palette.text_secondary,
                ),
            );
            paint_galley(ui, rect, label, palette.text_secondary);
        }
    }
}
