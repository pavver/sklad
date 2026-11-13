//! Laying out text for painting: one line, cut with an ellipsis, or wrapped to a width.

use std::sync::Arc;

use eframe::egui::text_selection::LabelSelectionState;
use eframe::egui::{
    Align, Color32, CursorIcon, FontSelection, Galley, Pos2, Rect, Response, RichText, Sense, Stroke,
    TextWrapMode, Ui, WidgetText, pos2,
};

/// Text on one line at its natural width.
pub fn galley(ui: &Ui, text: RichText) -> Arc<Galley> {
    WidgetText::from(text).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        FontSelection::Default,
    )
}

/// Text on one line, cut with an ellipsis at `width`.
pub fn truncated(ui: &Ui, text: RichText, width: f32) -> Arc<Galley> {
    laid_out(ui, text, TextWrapMode::Truncate, width)
}

/// Text wrapped at `width`, breaking anywhere when a word is longer.
pub fn wrapped(ui: &Ui, text: RichText, width: f32) -> Arc<Galley> {
    laid_out(ui, text, TextWrapMode::Wrap, width)
}

fn laid_out(ui: &Ui, text: RichText, mode: TextWrapMode, width: f32) -> Arc<Galley> {
    WidgetText::from(text).into_galley(ui, Some(mode), width.max(0.0), FontSelection::Default)
}

/// Paints text centred in `rect`, on whole pixels.
pub fn paint_galley(ui: &Ui, rect: Rect, galley: Arc<Galley>, colour: Color32) {
    let corner = (rect.center() - galley.size() / 2.0).round();
    ui.painter().galley(corner, galley, colour);
}

/// Paints text in `rect`, centred vertically and placed by `align` horizontally.
pub fn paint_aligned(ui: &Ui, rect: Rect, galley: Arc<Galley>, align: Align, colour: Color32) {
    let x = match align {
        Align::Min => rect.left(),
        Align::Center => rect.center().x - galley.size().x / 2.0,
        Align::Max => rect.right() - galley.size().x,
    };
    let y = rect.center().y - galley.size().y / 2.0;
    ui.painter().galley(pos2(x, y).round(), galley, colour);
}

/// What text that can be selected senses: a click and a drag, without taking focus from Tab.
pub fn selectable_sense() -> Sense {
    Sense::click_and_drag() - Sense::FOCUSABLE
}

/// Paints text the person can select and copy, as a selectable `Label` does. `response` must
/// come from `selectable_sense` over the rectangle of the text.
pub fn paint_selectable(ui: &Ui, response: &Response, at: Pos2, galley: Arc<Galley>, colour: Color32) {
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Text);
    }
    LabelSelectionState::label_text_selection(ui, response, at, galley, colour, Stroke::NONE);
}
