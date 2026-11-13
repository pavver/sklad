//! `Combobox` (`docs/components/combobox.md`): a text field with a filtered list of options.
//!
//! The field is a TextField with a trailing `arrow_drop_down`; the list is the DropdownMenu popup
//! under it. Custom values are not ported, since nothing here accepts them.

use eframe::egui::{
    self, Align2, CursorIcon, Id, Key, Margin, Modifiers, Order, Rect, Sense, TextEdit, Ui, WidgetInfo, pos2,
    vec2,
};
use operator_ui_tokens::motion;

use super::icon::{self, paint_icon};
use super::state::{ControlState, Look, Outline, paint_border};
use super::text::{galley, paint_aligned};
use super::tokens::{BOLD, MEDIUM, Palette, Type, dim, du, mix, secs, text, weighted};

pub struct Combobox<'a> {
    pub id: Id,
    pub options: &'a [String],
    /// The accessible name of the field.
    pub label: &'a str,
    /// Shown in the list when nothing matches the text.
    pub no_matches: &'a str,
    pub width: f32,
}

/// What lives between frames.
#[derive(Clone, Debug, Default)]
struct Memory {
    query: String,
    open: bool,
    /// Typing switched the list from all options to the matching ones.
    filtering: bool,
    active: usize,
    /// The popup of the last frame, so a press on it does not count as leaving the field.
    popup: Option<Rect>,
    /// The active option must be scrolled into view, in the middle when it is `Some(true)`.
    reveal: Option<bool>,
}

/// The width that fits the longest option, the paddings and the arrow.
pub fn natural_width(ui: &Ui, options: &[String]) -> f32 {
    let widest = options
        .iter()
        .map(|option| {
            galley(ui, weighted(option, Type::Body, BOLD, egui::Color32::WHITE))
                .size()
                .x
        })
        .fold(0.0, f32::max);
    (widest + 2.0 * du(dim::PAD_CARD) + du(dim::ROW_STANDARD)).ceil()
}

impl Combobox<'_> {
    /// Draws the field over the option at `selected`. Returns the option the person committed.
    pub fn show(&self, ui: &mut Ui, selected: usize, palette: &Palette) -> Option<usize> {
        let ctx = ui.ctx().clone();
        let key = self.id.with("memory");
        let mut memory: Memory = ctx.data(|data| data.get_temp(key)).unwrap_or_default();
        let committed = self.options.get(selected).cloned().unwrap_or_default();
        let field_id = self.id.with("field");
        let focused_before = ctx.memory(|m| m.has_focus(field_id));
        if !focused_before && !memory.open {
            memory.query.clone_from(&committed);
        }

        let height = du(dim::ROW_STANDARD);
        let (rect, _) = ui.allocate_exact_size(vec2(self.width, height), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, palette.surface);
        let arrow = Rect::from_min_max(pos2(rect.right() - height, rect.top()), rect.max);
        let mut chosen = None;
        if focused_before {
            let matches = self.matches(&memory);
            chosen = self.keys(ui, &mut memory, &matches, &committed);
        }
        let text_rect = Rect::from_min_max(
            pos2(rect.left() + du(dim::PAD_CARD), rect.top()),
            pos2(arrow.left(), rect.bottom()),
        );
        let response = self.field(ui, &mut memory.query, text_rect, palette);
        let arrow_response = ui.interact(arrow, self.id.with("arrow"), Sense::click());
        self.react(
            ui,
            &mut memory,
            &response,
            &arrow_response,
            &committed,
            &mut chosen,
        );
        // The look of this frame is what the field was before a pick in the list closes it.
        let focused = response.has_focus();
        let in_use = focused || memory.open;
        if memory.open {
            if let Some(pick) = self.popup(ui, rect, &mut memory, selected, palette) {
                memory.query.clone_from(&self.options[pick]);
                memory.open = false;
                memory.filtering = false;
                chosen = Some(pick);
                ctx.memory_mut(|m| m.surrender_focus(field_id));
            }
        } else {
            memory.popup = None;
        }
        self.paint_chrome(ui, rect, arrow, focused, in_use, palette);
        if response.hovered() {
            ctx.set_cursor_icon(CursorIcon::Text);
        }
        if arrow_response.hovered() {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        ctx.data_mut(|data| data.insert_temp(key, memory));
        chosen.filter(|&pick| pick != selected)
    }

    /// The text field, borderless, over the whole field but the arrow.
    fn field(&self, ui: &mut Ui, query: &mut String, rect: Rect, palette: &Palette) -> egui::Response {
        let edit = TextEdit::singleline(query)
            .id(self.id.with("field"))
            .font(Type::Body.font())
            .text_color(palette.text_primary)
            .frame(egui::Frame::NONE)
            .margin(Margin::ZERO)
            .desired_width(rect.width())
            .min_size(rect.size())
            .align(Align2::LEFT_CENTER);
        let row = ui.spacing().interact_size.y;
        ui.spacing_mut().interact_size.y = rect.height();
        // `place` leaves the layout cursor alone: the field already took its whole rectangle.
        let response = ui.place(rect, edit);
        ui.spacing_mut().interact_size.y = row;
        response.widget_info(|| WidgetInfo::labeled(egui::Role::ComboBox, true, self.label));
        response
    }

    /// Opens the list on a click or focus, filters it on typing, and settles the text when the
    /// field is left.
    fn react(
        &self,
        ui: &Ui,
        memory: &mut Memory,
        response: &egui::Response,
        arrow: &egui::Response,
        committed: &str,
        chosen: &mut Option<usize>,
    ) {
        if arrow.clicked() {
            response.request_focus();
        }
        if arrow.clicked() || response.gained_focus() || response.clicked() {
            self.open(memory, committed);
        }
        if response.changed() {
            memory.filtering = true;
            memory.active = 0;
            memory.open = true;
            memory.reveal = Some(false);
        }
        if left_the_field(ui, memory, response) {
            self.settle(memory, committed, chosen);
        }
    }

    /// The arrow, lit while the field is in use, and the border.
    fn paint_chrome(&self, ui: &Ui, rect: Rect, arrow: Rect, focused: bool, in_use: bool, palette: &Palette) {
        let lit = ui
            .ctx()
            .animate_bool_with_time(self.id.with("lit"), in_use, secs(motion::STATE_COLOR_MS));
        let glyph = mix(palette.text_secondary, palette.accent, lit);
        paint_icon(
            ui,
            icon::ARROW_DROP_DOWN,
            arrow.center(),
            du(dim::ICON_LARGE),
            glyph,
        );
        let state = if focused {
            ControlState::Focused
        } else {
            ControlState::Idle
        };
        paint_border(ui, rect, Outline::of(state, palette), &Look::IDLE, palette);
    }

    /// Indices of the options the list shows: all of them until the person types.
    fn matches(&self, memory: &Memory) -> Vec<usize> {
        let query = memory.query.to_lowercase();
        (0..self.options.len())
            .filter(|&index| {
                !memory.filtering || query.is_empty() || self.options[index].to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Opens the full list with the committed option active.
    fn open(&self, memory: &mut Memory, committed: &str) {
        if memory.open {
            return;
        }
        memory.filtering = false;
        memory.active = self
            .options
            .iter()
            .position(|option| option == committed)
            .unwrap_or(0);
        memory.open = true;
        memory.reveal = Some(true);
    }

    /// Leaving the field commits text that names an option exactly, and restores the rest.
    fn settle(&self, memory: &mut Memory, committed: &str, chosen: &mut Option<usize>) {
        memory.open = false;
        memory.filtering = false;
        match self.options.iter().position(|option| *option == memory.query) {
            Some(index) if memory.query != committed => *chosen = Some(index),
            Some(_) => {}
            None => memory.query = committed.to_owned(),
        }
    }

    /// Arrow keys, Enter and Escape, taken before the text field sees them.
    fn keys(&self, ui: &Ui, memory: &mut Memory, matches: &[usize], committed: &str) -> Option<usize> {
        let take = |key: Key| ui.input_mut(|input| input.consume_key(Modifiers::NONE, key));
        let count = matches.len();
        if take(Key::ArrowDown) {
            if !memory.open {
                self.open(memory, committed);
            } else if count > 0 {
                memory.active = (memory.active + 1) % count;
                memory.reveal = Some(false);
            }
        }
        if take(Key::ArrowUp) && memory.open && count > 0 {
            memory.active = (memory.active + count - 1) % count;
            memory.reveal = Some(false);
        }
        if take(Key::Escape) {
            memory.open = false;
            memory.filtering = false;
            committed.clone_into(&mut memory.query);
        }
        if take(Key::Enter) {
            return self.enter(memory, matches, committed);
        }
        None
    }

    /// Enter takes the active option of an open list, and settles the text otherwise.
    fn enter(&self, memory: &mut Memory, matches: &[usize], committed: &str) -> Option<usize> {
        let mut chosen = None;
        match matches.get(memory.active).copied().filter(|_| memory.open) {
            Some(index) => {
                memory.query.clone_from(&self.options[index]);
                memory.open = false;
                memory.filtering = false;
                chosen = Some(index);
            }
            None => self.settle(memory, committed, &mut chosen),
        }
        chosen
    }

    /// The DropdownMenu popup under the field. Returns the option clicked.
    fn popup(
        &self,
        ui: &Ui,
        field: Rect,
        memory: &mut Memory,
        selected: usize,
        palette: &Palette,
    ) -> Option<usize> {
        let matches = self.matches(memory);
        let query = if memory.filtering {
            memory.query.to_lowercase()
        } else {
            String::new()
        };
        let row = du(dim::ROW_CONTROL);
        let border = du(dim::BORDER);
        let max_height = du(50.0) - 2.0 * border;
        let mut clicked = None;
        let area = egui::Area::new(self.id.with("popup"))
            .order(Order::Foreground)
            .fixed_pos(field.left_bottom())
            .show(ui.ctx(), |ui| {
                let width = field.width();
                ui.set_width(width);
                let rows = matches.len().max(1) as f32;
                let inner = (rows * row).min(max_height);
                let (frame, _) = ui.allocate_exact_size(vec2(width, inner + 2.0 * border), Sense::hover());
                ui.painter().rect_filled(frame, 0.0, palette.surface);
                let body = frame.shrink(border);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
                let reveal = memory.reveal.take();
                egui::ScrollArea::vertical()
                    .id_salt(self.id.with("scroll"))
                    .max_height(inner)
                    .auto_shrink([false, true])
                    .show(&mut child, |ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        if matches.is_empty() {
                            self.no_matches_row(ui, body.width(), palette);
                        }
                        let rows = OptionRows {
                            width: body.width(),
                            selected,
                            query: &query,
                            reveal,
                        };
                        for (position, &index) in matches.iter().enumerate() {
                            if self.option_row(ui, position, index, &rows, memory, palette) {
                                clicked = Some(index);
                            }
                        }
                    });
                egui::Painter::rect_stroke(
                    ui.painter(),
                    frame.shrink(border / 2.0),
                    0.0,
                    egui::Stroke::new(border, palette.accent),
                    egui::StrokeKind::Middle,
                );
            });
        memory.popup = Some(area.response.rect);
        clicked
    }

    /// The row that says nothing matches the text.
    fn no_matches_row(&self, ui: &mut Ui, width: f32, palette: &Palette) {
        let (cell, _) = ui.allocate_exact_size(vec2(width, du(dim::ROW_CONTROL)), Sense::hover());
        let label = galley(ui, text(self.no_matches, Type::Body, palette.text_secondary));
        let place = cell.shrink2(vec2(du(dim::PAD_CARD), 0.0));
        paint_aligned(ui, place, label, egui::Align::Min, palette.text_secondary);
    }

    /// One option of the list: the pointer moving over it makes it active. Returns whether it
    /// was clicked.
    fn option_row(
        &self,
        ui: &mut Ui,
        position: usize,
        index: usize,
        rows: &OptionRows<'_>,
        memory: &mut Memory,
        palette: &Palette,
    ) -> bool {
        let (cell, response) = ui.allocate_exact_size(vec2(rows.width, du(dim::ROW_CONTROL)), Sense::click());
        if response.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
            memory.active = position;
        }
        let active = position == memory.active;
        let is_selected = index == rows.selected;
        let background = if active {
            palette.pressed
        } else if is_selected {
            mix(palette.surface, palette.accent, 0.08)
        } else {
            palette.surface
        };
        ui.painter().rect_filled(cell, 0.0, background);
        let option = &self.options[index];
        paint_option(ui, cell, option, rows.query, is_selected, palette);
        response.widget_info(|| WidgetInfo::selected(egui::Role::ListBoxOption, true, is_selected, option));
        if active && let Some(centre) = rows.reveal {
            ui.scroll_to_rect(cell, centre.then_some(egui::Align::Center));
        }
        response.on_hover_cursor(CursorIcon::PointingHand).clicked()
    }
}

/// Whether the person left the field this frame: focus went elsewhere, or a press was released
/// outside an open list. A press on the list itself does not count.
fn left_the_field(ui: &Ui, memory: &Memory, response: &egui::Response) -> bool {
    let on_popup = memory
        .popup
        .zip(ui.ctx().pointer_latest_pos())
        .is_some_and(|(popup, at)| popup.contains(at));
    let released_elsewhere =
        memory.open && !response.has_focus() && ui.input(|input| input.pointer.any_released());
    (response.lost_focus() || released_elsewhere) && !on_popup
}

/// What every option row of one list shares.
struct OptionRows<'a> {
    width: f32,
    selected: usize,
    query: &'a str,
    /// Scroll the active option into view, in the middle when `Some(true)`.
    reveal: Option<bool>,
}

/// The option text, with the part that matches the typed text in `accent` bold.
fn paint_option(ui: &Ui, cell: Rect, option: &str, query: &str, selected: bool, palette: &Palette) {
    let (colour, weight) = if selected {
        (palette.accent, BOLD)
    } else {
        (palette.text_primary, MEDIUM)
    };
    let lower = option.to_lowercase();
    let found = (!query.is_empty() && lower.len() == option.len())
        .then(|| lower.find(query))
        .flatten();
    let parts = match found {
        Some(start) => {
            let end = start + query.len();
            vec![
                (&option[..start], colour, weight),
                (&option[start..end], palette.accent, BOLD),
                (&option[end..], colour, weight),
            ]
        }
        None => vec![(option, colour, weight)],
    };
    let mut left = cell.left() + du(dim::PAD_CARD);
    for (part, colour, weight) in parts {
        if part.is_empty() {
            continue;
        }
        let piece = galley(ui, weighted(part, Type::Body, weight, colour));
        let width = piece.size().x;
        let at = Rect::from_min_max(pos2(left, cell.top()), pos2(cell.right(), cell.bottom()));
        paint_aligned(ui, at, piece, egui::Align::Min, colour);
        left += width;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::operator::tokens::LIGHT;
    use eframe::egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    const OPTIONS: [&str; 3] = ["English", "Français", "Українська"];

    /// The chosen index, which a choice from the field changes.
    fn harness() -> Harness<'static, usize> {
        let options: Vec<String> = OPTIONS.map(str::to_owned).to_vec();
        let mut harness = Harness::new_ui_state(
            move |ui, chosen: &mut usize| {
                let field = Combobox {
                    id: Id::unique("language"),
                    options: &options,
                    label: "Language",
                    no_matches: "Nothing",
                    width: 200.0,
                };
                if let Some(pick) = field.show(ui, *chosen, &LIGHT) {
                    *chosen = pick;
                }
            },
            0,
        );
        harness.run();
        harness
    }

    fn focus(harness: &mut Harness<'_, usize>) {
        harness.get_by_role(Role::ComboBox).focus();
        harness.run();
    }

    fn press(harness: &mut Harness<'_, usize>, key: Key) {
        harness.key_press(key);
        harness.run();
    }

    /// Replaces the text of the field, as typing over a selection does.
    fn type_over(harness: &mut Harness<'_, usize>, text: &str) {
        harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
        harness.run();
        harness.get_by_role(Role::ComboBox).type_text(text);
        harness.run();
    }

    #[test]
    fn arrows_walk_the_list_around_and_enter_chooses() {
        let mut harness = harness();
        focus(&mut harness);
        // Focus opens the list on the chosen option, the first.
        press(&mut harness, Key::ArrowDown);
        press(&mut harness, Key::ArrowUp);
        press(&mut harness, Key::ArrowUp);
        press(&mut harness, Key::Enter);
        assert_eq!(*harness.state(), 2, "up from the first wraps to the last");
    }

    #[test]
    fn typing_filters_and_enter_takes_the_first_match() {
        let mut harness = harness();
        focus(&mut harness);
        type_over(&mut harness, "ська");
        press(&mut harness, Key::Enter);
        assert_eq!(*harness.state(), 2);
    }

    #[test]
    fn escape_keeps_the_choice_and_restores_the_text() {
        let mut harness = harness();
        focus(&mut harness);
        type_over(&mut harness, "Fra");
        press(&mut harness, Key::Escape);
        press(&mut harness, Key::Enter);
        assert_eq!(*harness.state(), 0, "nothing chosen");
        assert_eq!(
            harness.get_by_role(Role::ComboBox).value().as_deref(),
            Some("English")
        );
    }

    #[test]
    fn leaving_the_field_commits_an_exact_name_and_drops_the_rest() {
        let mut harness = harness();
        focus(&mut harness);
        type_over(&mut harness, "xyz");
        press(&mut harness, Key::Enter);
        assert_eq!(*harness.state(), 0, "no option is called so");
        assert_eq!(
            harness.get_by_role(Role::ComboBox).value().as_deref(),
            Some("English")
        );
        focus(&mut harness);
        type_over(&mut harness, "Français");
        press(&mut harness, Key::Tab);
        assert_eq!(*harness.state(), 1, "the exact name of an option chooses it");
    }
}
