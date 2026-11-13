//! `SearchField` (`docs/components/search-field.md`): reports the query after a pause in typing.

use eframe::egui::{
    self, Align2, CursorIcon, Id, Key, Margin, Rect, Sense, TextEdit, Ui, WidgetInfo, pos2, vec2,
};
use operator_ui_tokens::motion;

use super::icon::{self, paint_icon};
use super::state::{ControlState, Look, Outline, hover_amount, paint_border};
use super::tokens::{Palette, Type, dim, du, mix, secs, text};

pub struct SearchField<'a> {
    pub id: Id,
    pub placeholder: &'a str,
    pub clear_label: &'a str,
    /// Height in DU: `10`, or `8` inside a toolbar.
    pub height: f32,
    pub width: f32,
}

impl SearchField<'_> {
    /// Draws the field over `query`. Returns the query when it is to be reported.
    pub fn show(&self, ui: &mut Ui, query: &mut String, palette: &Palette) -> Option<String> {
        let height = du(self.height);
        let (rect, _) = ui.allocate_exact_size(vec2(self.width, height), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, palette.surface);

        let clear_width = if query.is_empty() { 0.0 } else { height };
        let text_rect = Rect::from_min_max(
            pos2(rect.left() + height, rect.top()),
            pos2(
                rect.right() - clear_width - du(dim::PAD_CARD) * f32::from(query.is_empty()),
                rect.bottom(),
            ),
        );
        let edit = TextEdit::singleline(query)
            .id(self.id)
            .hint_text(text(self.placeholder, Type::Body, palette.text_disabled))
            .font(Type::Body.font())
            .text_color(palette.text_primary)
            .frame(egui::Frame::NONE)
            .margin(Margin::ZERO)
            .desired_width(text_rect.width())
            .min_size(text_rect.size())
            .align(Align2::LEFT_CENTER);
        let row = ui.spacing().interact_size.y;
        ui.spacing_mut().interact_size.y = height;
        // `place` leaves the layout cursor alone: the field already took its whole rectangle.
        let response = ui.place(text_rect, edit);
        ui.spacing_mut().interact_size.y = row;
        let focused = response.has_focus();
        let mut report_now = focused && ui.input(|input| input.key_pressed(Key::Enter));
        let mut changed = response.changed();
        if focused && !query.is_empty() && ui.input(|input| input.key_pressed(Key::Escape)) {
            query.clear();
            changed = true;
            report_now = true;
        }

        let lit = ui
            .ctx()
            .animate_bool_with_time(self.id.with("lit"), focused, secs(motion::STATE_COLOR_MS));
        let slot = Rect::from_min_size(rect.min, vec2(height, height));
        let glyph = mix(palette.text_secondary, palette.accent, lit);
        paint_icon(ui, icon::SEARCH, slot.center(), du(dim::ICON_LARGE), glyph);

        if clear_width > 0.0 {
            let cell = Rect::from_min_max(pos2(rect.right() - clear_width, rect.top()), rect.max);
            let clear = ui.interact(cell, self.id.with("clear"), Sense::click());
            clear.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, self.clear_label));
            let hover = hover_amount(ui, &clear);
            let colour = mix(palette.icon_secondary, palette.icon, hover);
            paint_icon(ui, icon::CLOSE, cell.center(), du(dim::ICON_SMALL), colour);
            let clear = clear.on_hover_cursor(CursorIcon::PointingHand);
            if clear.clicked() {
                query.clear();
                changed = true;
                report_now = true;
                response.request_focus();
            }
        }

        let state = if focused {
            ControlState::Focused
        } else {
            ControlState::Idle
        };
        paint_border(ui, rect, Outline::of(state, palette), &Look::IDLE, palette);
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
        }
        self.debounce(ui, query, changed, report_now)
    }

    /// The query is reported `search_debounce` after the last change, or at once on Enter.
    fn debounce(&self, ui: &Ui, query: &str, changed: bool, now: bool) -> Option<String> {
        let key = self.id.with("changed at");
        let time = ui.input(|input| input.time);
        if now {
            ui.ctx().data_mut(|data| data.remove::<f64>(key));
            return Some(query.to_owned());
        }
        if changed {
            ui.ctx().data_mut(|data| data.insert_temp(key, time));
        }
        let since: Option<f64> = ui.ctx().data(|data| data.get_temp(key));
        let pause = f64::from(secs(motion::SEARCH_DEBOUNCE_MS));
        match since {
            Some(at) if time - at >= pause => {
                ui.ctx().data_mut(|data| data.remove::<f64>(key));
                Some(query.to_owned())
            }
            Some(at) => {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(pause - (time - at)));
                None
            }
            None => None,
        }
    }
}
