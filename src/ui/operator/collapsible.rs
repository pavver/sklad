//! `Collapsible` (`docs/components/accordion.md`): a section with a header row that shows or
//! hides its content.

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{self, CursorIcon, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};
use operator_ui_tokens::motion;

use super::icon::{self, paint_icon_turned};
use super::layout::{Tone, counter_size, paint_counter};
use super::motion::standard;
use super::state::{hover_amount, paint_focus_ring};
use super::text::{paint_aligned, truncated};
use super::tokens::{MEDIUM, Palette, Type, dim, du, mix, secs, tint, weighted};

/// One section.
pub struct Collapsible<'a> {
    pub id: Id,
    pub title: &'a str,
    /// A `Counter` at the end of the header, hidden at zero.
    pub count: usize,
    /// The largest number the counter shows in full.
    pub counter_max: usize,
    pub initially_open: bool,
}

impl Collapsible<'_> {
    fn open_key(&self) -> Id {
        self.id.with("open")
    }

    /// Whether the section is open, as the person left it.
    pub fn is_open(&self, ui: &Ui) -> bool {
        ui.ctx()
            .data(|data| data.get_temp(self.open_key()))
            .unwrap_or(self.initially_open)
    }

    /// Draws the header and, while the section is open or closing, the content.
    pub fn show(&self, ui: &mut Ui, palette: &Palette, content: impl FnOnce(&mut Ui)) {
        let mut open = self.is_open(ui);
        if self.header(ui, open, palette) {
            open = !open;
            ui.ctx().data_mut(|data| data.insert_temp(self.open_key(), open));
        }
        let openness = self.openness(ui, open);
        if openness <= 0.0 {
            return;
        }

        // The content is indented to the title: past the start padding, the chevron and the gap.
        let indent = du(2.0 * dim::GUTTER + dim::ICON_SMALL);
        let (top, bottom, end) = (du(dim::PAD_TIGHT), du(dim::PAD_CARD), du(dim::PAD_CARD));
        let width = ui.available_width();
        let origin = ui.cursor().min;
        let height_key = self.id.with("height");
        // The clip uses the height measured in the previous frame: the shapes are clipped as they
        // are added, before this frame's height is known.
        let known: Option<f32> = ui.ctx().data(|data| data.get_temp(height_key));
        let clip_height = known.map_or(f32::INFINITY, |full| full * openness);
        let clip = Rect::from_min_size(origin, vec2(width, clip_height)).intersect(ui.clip_rect());
        let inner = Rect::from_min_size(
            pos2(origin.x + indent, origin.y + top),
            vec2((width - indent - end).max(0.0), f32::INFINITY),
        );
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
        child.set_clip_rect(clip);
        if openness < 1.0 {
            child.disable();
        }
        content(&mut child);
        let full = top + child.min_rect().height() + bottom;
        ui.ctx().data_mut(|data| data.insert_temp(height_key, full));
        ui.allocate_exact_size(vec2(width, full * openness), Sense::hover());
    }

    /// The share of the content shown, `0` closed to `1` open, eased over `expand`.
    fn openness(&self, ui: &Ui, open: bool) -> f32 {
        ui.ctx().animate_bool_with_time_and_easing(
            self.id.with("openness"),
            open,
            secs(motion::EXPAND_MS),
            standard,
        )
    }

    /// The header row. Returns whether it was clicked.
    fn header(&self, ui: &mut Ui, open: bool, palette: &Palette) -> bool {
        let size = vec2(ui.available_width(), du(dim::ROW_STANDARD));
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let response = ui.interact(rect, self.id, Sense::click());
        let title = self.title;
        response.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, open, title));
        if ui.is_rect_visible(rect) {
            let hover = hover_amount(ui, &response);
            ui.painter().rect_filled(rect, 0.0, tint(palette.hover, hover));
            let turn = self.openness(ui, open);
            let chevron = du(dim::ICON_SMALL);
            let start = rect.left() + du(dim::GUTTER);
            let colour = mix(palette.text_secondary, palette.accent, turn);
            let centre = pos2(start + chevron / 2.0, rect.center().y);
            paint_icon_turned(ui, icon::CHEVRON_RIGHT, centre, chevron, colour, turn * FRAC_PI_2);

            let left = start + chevron + du(dim::GUTTER);
            let mut right = rect.right() - du(dim::PAD_CARD);
            if self.count > 0 {
                let badge = counter_size(ui, self.count, self.counter_max);
                let at = pos2(right - badge.x, rect.center().y - badge.y / 2.0);
                let badge_rect = Rect::from_min_size(at, badge);
                paint_counter(
                    ui,
                    badge_rect,
                    self.count,
                    self.counter_max,
                    Tone::Neutral,
                    palette,
                );
                right = badge_rect.left() - du(dim::GUTTER);
            }
            let words = weighted(self.title, Type::Body, MEDIUM, palette.text_primary);
            let words = truncated(ui, words, (right - left).max(0.0));
            let text_rect = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom()));
            paint_aligned(ui, text_rect, words, egui::Align::Min, palette.text_primary);
            paint_focus_ring(ui, &response, rect, palette);
        }
        response.on_hover_cursor(CursorIcon::PointingHand).clicked()
    }
}
