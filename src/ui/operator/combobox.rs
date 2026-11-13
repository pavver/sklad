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
        let text_rect = Rect::from_min_max(
            pos2(rect.left() + du(dim::PAD_CARD), rect.top()),
            pos2(arrow.left(), rect.bottom()),
        );

        let mut chosen = None;
        let matches = self.matches(&memory);
        if focused_before {
            chosen = self.keys(ui, &mut memory, &matches, &committed);
        }

        let edit = TextEdit::singleline(&mut memory.query)
            .id(field_id)
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
        response.widget_info(|| WidgetInfo::labeled(egui::Role::ComboBox, true, self.label));
        let arrow_response = ui.interact(arrow, self.id.with("arrow"), Sense::click());
        if arrow_response.clicked() {
            response.request_focus();
            self.open(&mut memory, &committed);
        }
        if response.gained_focus() || response.clicked() {
            self.open(&mut memory, &committed);
        }
        if response.changed() {
            memory.filtering = true;
            memory.active = 0;
            memory.open = true;
            memory.reveal = Some(false);
        }

        let pointer_on_popup = memory
            .popup
            .zip(ctx.pointer_latest_pos())
            .is_some_and(|(popup, at)| popup.contains(at));
        if response.lost_focus() && !pointer_on_popup {
            self.settle(&mut memory, &committed, &mut chosen);
        }
        let focused = response.has_focus();
        if memory.open && !focused && !pointer_on_popup && ctx.input(|i| i.pointer.any_released()) {
            self.settle(&mut memory, &committed, &mut chosen);
        }

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

        let lit = ctx.animate_bool_with_time(
            self.id.with("lit"),
            focused || memory.open,
            secs(motion::STATE_COLOR_MS),
        );
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
        if response.hovered() {
            ctx.set_cursor_icon(CursorIcon::Text);
        }
        if arrow_response.hovered() {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        ctx.data_mut(|data| data.insert_temp(key, memory));
        chosen.filter(|&pick| pick != selected)
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
            let pick = matches.get(memory.active).copied().filter(|_| memory.open);
            let mut chosen = None;
            match pick {
                Some(index) => {
                    memory.query.clone_from(&self.options[index]);
                    memory.open = false;
                    memory.filtering = false;
                    chosen = Some(index);
                }
                None => self.settle(memory, committed, &mut chosen),
            }
            return chosen;
        }
        None
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
                            let (cell, _) = ui.allocate_exact_size(vec2(body.width(), row), Sense::hover());
                            let label = galley(ui, text(self.no_matches, Type::Body, palette.text_secondary));
                            paint_aligned(
                                ui,
                                cell.shrink2(vec2(du(dim::PAD_CARD), 0.0)),
                                label,
                                egui::Align::Min,
                                palette.text_secondary,
                            );
                        }
                        for (position, &index) in matches.iter().enumerate() {
                            let (cell, response) =
                                ui.allocate_exact_size(vec2(body.width(), row), Sense::click());
                            if response.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                                memory.active = position;
                            }
                            let is_selected = index == selected;
                            let background = if position == memory.active {
                                palette.pressed
                            } else if is_selected {
                                mix(palette.surface, palette.accent, 0.08)
                            } else {
                                palette.surface
                            };
                            ui.painter().rect_filled(cell, 0.0, background);
                            paint_option(ui, cell, &self.options[index], &query, is_selected, palette);
                            response.widget_info(|| {
                                WidgetInfo::selected(
                                    egui::Role::ListBoxOption,
                                    true,
                                    is_selected,
                                    &self.options[index],
                                )
                            });
                            if position == memory.active
                                && let Some(centre) = reveal
                            {
                                let align = if centre { Some(egui::Align::Center) } else { None };
                                ui.scroll_to_rect(cell, align);
                            }
                            if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
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
