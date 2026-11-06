//! The list of items: rows of equal height, so only the visible ones are laid out.

use eframe::egui::{
    self, Align, Label, Layout, Rect, RichText, ScrollArea, Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2,
    vec2,
};

use super::format;
use super::theme::{AVATAR_ROW, BORDER, BORDER_THICK, GUTTER, PAD_CARD, Palette, Type, du, styled, tint};
use super::widgets::{IconCache, avatar, hover_amount, row_id};
use crate::i18n::Text;
use crate::model::{Item, Usage};

const SIZE_COLUMN: f32 = du(22.0);
const RUN_COLUMN: f32 = du(32.0);

/// One line of text in a rectangle, cut with an ellipsis when it does not fit.
pub fn line_in(ui: &mut Ui, rect: Rect, text: RichText, align: Align) {
    let layout = match align {
        Align::Max => Layout::right_to_left(Align::Center),
        _ => Layout::left_to_right(Align::Center),
    };
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(layout), |ui| {
        ui.add(Label::new(text).truncate());
    });
}

pub struct Rows<'a> {
    pub items: &'a [Item],
    pub indices: &'a [usize],
    pub selected: Option<&'a str>,
    pub now: i64,
    pub text: &'static Text,
}

fn usage_colour(usage: Usage, palette: &Palette) -> egui::Color32 {
    match usage {
        Usage::Running => palette.success,
        Usage::NoRecord => palette.pending,
        Usage::Last { .. } => palette.text_primary,
        Usage::NotTracked => palette.text_disabled,
    }
}

fn header(ui: &mut Ui, text: &Text, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), du(6.0)), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.surface_alt);
    let colour = palette.text_secondary;
    let size_rect = Rect::from_min_max(
        pos2(rect.right() - RUN_COLUMN - SIZE_COLUMN, rect.top()),
        pos2(rect.right() - RUN_COLUMN - GUTTER, rect.bottom()),
    );
    let run_rect = Rect::from_min_max(
        pos2(rect.right() - RUN_COLUMN, rect.top()),
        pos2(rect.right() - PAD_CARD, rect.bottom()),
    );
    let name_rect = Rect::from_min_max(
        pos2(rect.left() + PAD_CARD, rect.top()),
        pos2(size_rect.left(), rect.bottom()),
    );
    line_in(
        ui,
        name_rect,
        styled(text.column_program, Type::Subtitle, colour),
        Align::Min,
    );
    line_in(
        ui,
        size_rect,
        styled(text.column_size, Type::Subtitle, colour),
        Align::Max,
    );
    line_in(
        ui,
        run_rect,
        styled(text.column_last_run, Type::Subtitle, colour),
        Align::Max,
    );
}

fn row(ui: &mut Ui, item: &Item, rows: &Rows<'_>, icons: &mut IconCache, palette: &Palette) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), AVATAR_ROW), Sense::hover());
    let response = ui.interact(rect, row_id(&item.id), Sense::click());
    let selected = rows.selected == Some(item.id.as_str());
    response.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, selected, item.title.clone()));

    if selected {
        ui.painter().rect_filled(rect, 0.0, tint(palette.accent, 0.15));
        let marker = Rect::from_min_size(rect.min, vec2(BORDER_THICK, rect.height()));
        ui.painter().rect_filled(marker, 0.0, palette.accent);
    } else {
        ui.painter()
            .rect_filled(rect, 0.0, tint(palette.surface, hover_amount(ui, &response)));
    }
    let bottom = rect.bottom() - BORDER / 2.0;
    ui.painter().hline(
        rect.x_range(),
        bottom,
        Stroke::new(BORDER, palette.disabled_border),
    );

    let icon_rect = Rect::from_center_size(
        pos2(rect.left() + PAD_CARD + du(5.0), rect.center().y),
        vec2(du(10.0), du(10.0)),
    );
    avatar(ui, icon_rect, item.icon.as_deref(), &item.title, icons, palette);

    let text_left = icon_rect.right() + PAD_CARD;
    let text_right = rect.right() - RUN_COLUMN - SIZE_COLUMN;
    let middle = rect.center().y;
    let title_rect = Rect::from_min_max(
        pos2(text_left, middle - du(5.5)),
        pos2(text_right, middle + du(0.5)),
    );
    let summary_rect = Rect::from_min_max(
        pos2(text_left, middle + du(0.5)),
        pos2(text_right, middle + du(5.0)),
    );
    line_in(
        ui,
        title_rect,
        styled(item.title.clone(), Type::RowTitle, palette.text_primary),
        Align::Min,
    );
    let summary = if item.summary.is_empty() {
        &item.package
    } else {
        &item.summary
    };
    line_in(
        ui,
        summary_rect,
        styled(summary.clone(), Type::Subtitle, palette.text_secondary),
        Align::Min,
    );

    let size_rect = Rect::from_min_max(
        pos2(text_right, rect.top()),
        pos2(rect.right() - RUN_COLUMN - GUTTER, rect.bottom()),
    );
    let size = item
        .size
        .map_or_else(|| "—".to_owned(), |size| format::size(size, rows.text));
    line_in(
        ui,
        size_rect,
        styled(size, Type::Console, palette.text_secondary),
        Align::Max,
    );

    let run_rect = Rect::from_min_max(
        pos2(rect.right() - RUN_COLUMN, rect.top()),
        pos2(rect.right() - PAD_CARD, rect.bottom()),
    );
    let usage = format::usage(item.usage, rows.now, rows.text);
    line_in(
        ui,
        run_rect,
        styled(usage, Type::Console, usage_colour(item.usage, palette)),
        Align::Max,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Draws the list and returns the id of the row clicked.
pub fn show(ui: &mut Ui, rows: &Rows<'_>, icons: &mut IconCache, palette: &Palette) -> Option<String> {
    header(ui, rows.text, palette);
    let mut clicked = None;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical().auto_shrink([false, false]).show_rows(
        ui,
        AVATAR_ROW,
        rows.indices.len(),
        |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for &index in &rows.indices[range] {
                let item = &rows.items[index];
                if row(ui, item, rows, icons, palette) {
                    clicked = Some(item.id.clone());
                }
            }
        },
    );
    clicked
}
