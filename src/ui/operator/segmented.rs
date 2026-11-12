//! `SegmentedControl`, synchronous variant (`docs/components/segmented-control.md`).

use eframe::egui::{self, CursorIcon, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2};
use operator_ui_tokens::motion;

use super::state::{BORDER, hover_amount, paint_focus_ring};
use super::text::{galley, paint_galley};
use super::tokens::{BOLD, MEDIUM, Palette, Type, dim, du, mix, secs, tint, weighted};

/// One option: the text on screen and its accessible name.
#[derive(Clone, Copy, Debug)]
pub struct Segment<'a> {
    pub text: &'a str,
    pub name: &'a str,
}

impl<'a> Segment<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, name: text }
    }

    pub fn named(text: &'a str, name: &'a str) -> Self {
        Self { text, name }
    }
}

/// The width at which every item holds its text: equal items, as wide as the widest one.
pub fn natural_width(ui: &Ui, items: &[Segment<'_>]) -> f32 {
    let widest = items
        .iter()
        .map(|item| {
            galley(
                ui,
                weighted(item.text, Type::BodySmall, BOLD, egui::Color32::WHITE),
            )
            .size()
            .x
        })
        .fold(0.0, f32::max);
    ((widest + 2.0 * du(dim::PAD_CARD)) * items.len() as f32 + 2.0 * du(dim::GAP_MICRO)).ceil()
}

/// Draws the control `width` wide and `height` DU high. Returns the index clicked.
pub fn segmented(
    ui: &mut Ui,
    id: Id,
    items: &[Segment<'_>],
    selected: usize,
    width: f32,
    height: f32,
    palette: &Palette,
) -> Option<usize> {
    let size = vec2(width, du(height));
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.surface_alt);
    let track = rect.shrink(du(dim::GAP_MICRO));
    let count = items.len().max(1) as f32;
    let cell_width = track.width() / count;
    let cell = |index: usize| {
        Rect::from_min_size(
            pos2(track.left() + cell_width * index as f32, track.top()),
            vec2(cell_width, track.height()),
        )
    };

    let slide = secs(motion::SEGMENT_SLIDE_MS);
    let target = cell(selected.min(items.len().saturating_sub(1)));
    let ctx = ui.ctx().clone();
    let left = ctx.animate_value_with_time(id.with("indicator"), target.left(), slide);
    let indicator = Rect::from_min_size(pos2(left, track.top()), target.size());

    let mut clicked = None;
    let mut responses = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let chosen = index == selected;
        let response = ui.interact(cell(index), id.with(index), Sense::click());
        response.widget_info(|| WidgetInfo::selected(egui::Role::Tab, true, chosen, item.name));
        if !chosen {
            let hover = hover_amount(ui, &response);
            ui.painter()
                .rect_filled(cell(index), 0.0, tint(palette.hover, hover));
        }
        let response = if chosen {
            response
        } else {
            response.on_hover_cursor(CursorIcon::PointingHand)
        };
        if response.clicked() && !chosen {
            clicked = Some(index);
        }
        responses.push(response);
    }

    ui.painter()
        .rect_filled(indicator, 0.0, tint(palette.accent, 0.15));
    let accent = Stroke::new(BORDER, palette.accent);
    ui.painter()
        .rect_stroke(indicator, 0.0, accent, StrokeKind::Inside);

    for (index, item) in items.iter().enumerate() {
        let lit = ctx.animate_bool_with_time(id.with(("lit", index)), index == selected, slide);
        let colour = mix(palette.text_secondary, palette.accent, lit);
        let weight = if index == selected { BOLD } else { MEDIUM };
        let text = galley(ui, weighted(item.text, Type::BodySmall, weight, colour));
        paint_galley(ui, cell(index), text, colour);
        paint_focus_ring(ui, &responses[index], cell(index), palette);
    }

    let outline = Stroke::new(BORDER, palette.border);
    ui.painter().rect_stroke(rect, 0.0, outline, StrokeKind::Inside);
    clicked
}
