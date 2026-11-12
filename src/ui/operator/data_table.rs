//! `DataTable` (draft, `docs/components/data-table.md`): text cells, sortable columns, a sticky
//! header and single selection. Rows have one height, so only the visible ones are laid out.

use eframe::egui::{
    self, Align, CursorIcon, Id, Key, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2,
};

use super::feedback::paint_indeterminate;
use super::icon::{self, paint_icon};
use super::state::{BORDER, BORDER_THICK, hover_amount, paint_focus_ring};
use super::text::{paint_aligned, truncated};
use super::tokens::{Palette, Type, dim, du, mix, text, tint};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Width {
    /// Fixed, in DU.
    Fixed(f32),
    /// A share of the width the fixed columns leave.
    Share(f32),
}

#[derive(Clone, Copy, Debug)]
pub struct Column<'a> {
    pub title: &'a str,
    pub width: Width,
    pub numeric: bool,
    pub sortable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Ascending,
    Descending,
}

/// The sorted column, or `None` for the original order.
pub type Sorting = Option<(usize, Direction)>;

/// A header click cycles ascending, descending, original order.
pub fn next_sorting(current: Sorting, column: usize) -> Sorting {
    match current {
        Some((sorted, Direction::Ascending)) if sorted == column => Some((column, Direction::Descending)),
        Some((sorted, Direction::Descending)) if sorted == column => None,
        _ => Some((column, Direction::Ascending)),
    }
}

pub struct DataTable<'a> {
    pub id: Id,
    pub columns: &'a [Column<'a>],
    pub rows: usize,
    pub sorting: Sorting,
    pub selected: Option<usize>,
    pub loading: bool,
}

#[derive(Default)]
pub struct Events {
    pub activated: Option<usize>,
    pub sorting: Option<Sorting>,
}

const HEADER: f32 = du(dim::ROW_COMPACT);
const ROW: f32 = du(dim::ROW_STANDARD);
const CELL_PAD: f32 = du(dim::PAD_CARD);

impl DataTable<'_> {
    fn spans(&self, left: f32, width: f32) -> Vec<(f32, f32)> {
        let fixed: f32 = self
            .columns
            .iter()
            .map(|column| match column.width {
                Width::Fixed(units) => du(units),
                Width::Share(_) => 0.0,
            })
            .sum();
        let shares: f32 = self
            .columns
            .iter()
            .map(|column| match column.width {
                Width::Share(share) => share,
                Width::Fixed(_) => 0.0,
            })
            .sum();
        let rest = (width - fixed).max(0.0);
        let mut x = left;
        self.columns
            .iter()
            .map(|column| {
                let span = match column.width {
                    Width::Fixed(units) => du(units),
                    Width::Share(share) => rest * share / shares.max(f32::EPSILON),
                };
                let cell = (x, x + span);
                x += span;
                cell
            })
            .collect()
    }

    /// Draws the table. `cell(row, column)` gives the text of a cell, `name(row)` the accessible
    /// name of a row, and `empty` what stands in place of no rows.
    pub fn show(
        &self,
        ui: &mut Ui,
        palette: &Palette,
        cell: impl Fn(usize, usize) -> String,
        name: impl Fn(usize) -> String,
        empty: impl FnOnce(&mut Ui),
    ) -> Events {
        let mut events = Events::default();
        let outer = ui.available_rect_before_wrap();
        let inner = outer.shrink(BORDER);
        let spans = self.spans(inner.left(), inner.width());

        let header = Rect::from_min_size(inner.min, vec2(inner.width(), HEADER));
        ui.allocate_rect(Rect::from_min_max(outer.min, header.max), Sense::hover());
        self.header(ui, header, &spans, palette, &mut events);
        if self.loading {
            let bar = Rect::from_min_size(header.left_bottom(), vec2(header.width(), BORDER_THICK));
            paint_indeterminate(ui, bar, palette);
        }

        let body = Rect::from_min_max(pos2(inner.left(), header.bottom()), inner.max);
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            if self.rows == 0 {
                ui.allocate_rect(body, Sense::hover());
                ui.scope_builder(egui::UiBuilder::new().max_rect(body), empty);
                return;
            }
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ScrollArea::vertical()
                .id_salt(self.id.with("body"))
                .auto_shrink([false, false])
                .show_rows(ui, ROW, self.rows, |ui, range| {
                    for row in range {
                        if self.row(ui, row, &spans, palette, &cell, &name) {
                            events.activated = Some(row);
                        }
                    }
                });
        });
        ui.painter().rect_stroke(
            outer,
            0.0,
            Stroke::new(BORDER, palette.border),
            StrokeKind::Inside,
        );
        events
    }

    fn header(&self, ui: &mut Ui, rect: Rect, spans: &[(f32, f32)], palette: &Palette, events: &mut Events) {
        ui.painter().rect_filled(rect, 0.0, palette.surface_alt);
        let arrow = du(3.5);
        for (index, (column, (left, right))) in self.columns.iter().zip(spans).enumerate() {
            let cell = Rect::from_x_y_ranges(*left..=*right, rect.y_range());
            let sorted = self
                .sorting
                .and_then(|(at, direction)| (at == index).then_some(direction));
            let mut hover = 0.0;
            if column.sortable {
                let response = ui.interact(cell, self.id.with(("sort", index)), Sense::click());
                let label = column.title;
                response
                    .widget_info(|| WidgetInfo::selected(egui::Role::Button, true, sorted.is_some(), label));
                hover = hover_amount(ui, &response);
                let response = response.on_hover_cursor(CursorIcon::PointingHand);
                if response.clicked() {
                    events.sorting = Some(next_sorting(self.sorting, index));
                }
                paint_focus_ring(ui, &response, cell, palette);
            }
            let content = cell.shrink2(vec2(CELL_PAD, 0.0));
            let room = (content.width() - arrow - du(dim::PAD_TIGHT)).max(0.0);
            let title = column.title.to_uppercase();
            let label = truncated(ui, text(title, Type::Caption, palette.text_secondary), room);
            let label_width = label.size().x;
            let align = if column.numeric { Align::Max } else { Align::Min };
            let text_rect = if column.numeric {
                Rect::from_min_max(
                    content.min,
                    pos2(content.right() - arrow - du(dim::PAD_TIGHT), content.bottom()),
                )
            } else {
                content
            };
            paint_aligned(ui, text_rect, label, align, palette.text_secondary);
            let arrow_x = if column.numeric {
                content.right() - arrow / 2.0
            } else {
                content.left() + label_width + du(dim::PAD_TIGHT) + arrow / 2.0
            };
            let centre = pos2(arrow_x, cell.center().y);
            match sorted {
                Some(direction) => {
                    let glyph = match direction {
                        Direction::Ascending => icon::ARROW_UPWARD,
                        Direction::Descending => icon::ARROW_DOWNWARD,
                    };
                    paint_icon(ui, glyph, centre, arrow, palette.accent);
                }
                None if hover > 0.0 => {
                    paint_icon(
                        ui,
                        icon::ARROW_UPWARD,
                        centre,
                        arrow,
                        tint(palette.icon, 0.5 * hover),
                    );
                }
                None => {}
            }
        }
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - BORDER / 2.0,
            Stroke::new(BORDER, palette.border),
        );
    }

    fn row(
        &self,
        ui: &mut Ui,
        row: usize,
        spans: &[(f32, f32)],
        palette: &Palette,
        cell: &impl Fn(usize, usize) -> String,
        name: &impl Fn(usize) -> String,
    ) -> bool {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
        let response = ui.interact(rect, self.id.with(("row", row)), Sense::click());
        let selected = self.selected == Some(row);
        response.widget_info(|| WidgetInfo::selected(egui::Role::Row, true, selected, name(row)));
        let activated = response.clicked()
            || (response.has_focus()
                && ui.input(|input| input.key_pressed(Key::Enter) || input.key_pressed(Key::Space)));
        if !ui.is_rect_visible(rect) {
            return activated;
        }
        if selected {
            ui.painter().rect_filled(rect, 0.0, palette.selection);
        } else {
            let hover = hover_amount(ui, &response);
            ui.painter()
                .rect_filled(rect, 0.0, mix(palette.background, palette.pressed, hover));
        }
        for (index, (column, (left, right))) in self.columns.iter().zip(spans).enumerate() {
            let cell_rect =
                Rect::from_x_y_ranges(*left..=*right, rect.y_range()).shrink2(vec2(CELL_PAD, 0.0));
            let galley = truncated(
                ui,
                text(cell(row, index), Type::BodySmall, palette.text_primary),
                cell_rect.width(),
            );
            let align = if column.numeric { Align::Max } else { Align::Min };
            paint_aligned(ui, cell_rect, galley, align, palette.text_primary);
        }
        if selected && let Some((left, _)) = spans.first() {
            let bar = Rect::from_min_size(pos2(*left, rect.top()), vec2(BORDER_THICK, rect.height()));
            ui.painter().rect_filled(bar, 0.0, palette.accent);
        }
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - BORDER / 2.0,
            Stroke::new(BORDER, palette.divider),
        );
        paint_focus_ring(ui, &response, rect.shrink(BORDER_THICK), palette);
        let _ = response.on_hover_cursor(CursorIcon::PointingHand);
        activated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_cycles_up_down_and_back_to_the_original_order() {
        let up = next_sorting(None, 2);
        assert_eq!(up, Some((2, Direction::Ascending)));
        let down = next_sorting(up, 2);
        assert_eq!(down, Some((2, Direction::Descending)));
        assert_eq!(next_sorting(down, 2), None);
        assert_eq!(next_sorting(down, 0), Some((0, Direction::Ascending)));
    }
}
