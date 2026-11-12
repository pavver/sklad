//! Laying out text for painting: one line, cut with an ellipsis, or wrapped to a width.

use std::sync::Arc;

use eframe::egui::{
    Align, Color32, FontSelection, Galley, Rect, RichText, TextWrapMode, Ui, WidgetText, pos2,
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
    WidgetText::from(text).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        width.max(0.0),
        FontSelection::Default,
    )
}

/// Text wrapped at `width`, breaking anywhere when a word is longer.
pub fn wrapped(ui: &Ui, text: RichText, width: f32) -> Arc<Galley> {
    WidgetText::from(text).into_galley(
        ui,
        Some(TextWrapMode::Wrap),
        width.max(0.0),
        FontSelection::Default,
    )
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
