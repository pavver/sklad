//! `SplitPane`, side by side: two panes and a divider between them.

use eframe::egui::{self, CursorIcon, Id, Key, Rect, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::state::{BORDER, BORDER_THICK};
use super::tokens::{Palette, dim, du};

/// The two panes of a `SplitPane`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Start,
    End,
}

/// `SplitPane`, side by side: two panes and a divider that sets the share of the first.
pub struct SplitPane {
    pub id: Id,
    pub initial: f32,
    /// Minimum pane size in DU.
    pub min_size: f32,
}

impl SplitPane {
    /// Draws both panes. Returns true when the person moved the divider; a share the clamp
    /// alone changed, because the window got narrower, does not count.
    pub fn show(
        &self,
        ui: &mut Ui,
        ratio: &mut f32,
        palette: &Palette,
        mut pane: impl FnMut(&mut Ui, Pane),
    ) -> bool {
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, Sense::hover());
        let minimum = du(self.min_size).min(rect.width() / 2.0);
        let clamp = |share: f32| {
            let lowest = minimum / rect.width().max(1.0);
            share.clamp(lowest, 1.0 - lowest)
        };
        *ratio = clamp(*ratio);
        let clamped = *ratio;
        let hit = du(dim::GUTTER);
        let x = rect.left() + rect.width() * *ratio;
        let handle = Rect::from_x_y_ranges(x - hit / 2.0..=x + hit / 2.0, rect.y_range());
        let response = ui.interact(handle, self.id.with("divider"), Sense::click_and_drag());
        let percent = (*ratio * 100.0).round();
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Splitter, true, format!("{percent}%")));
        if let Some(share) = self.asked_share(ui, &response, rect, *ratio) {
            *ratio = clamp(share);
        }
        let x = (rect.left() + rect.width() * *ratio).round();
        let first = Rect::from_min_max(rect.min, pos2(x, rect.bottom()));
        let second = Rect::from_min_max(pos2(x, rect.top()), rect.max);
        ui.scope_builder(UiBuilder::new().max_rect(first), |ui| {
            ui.set_clip_rect(first);
            pane(ui, Pane::Start);
        });
        ui.scope_builder(UiBuilder::new().max_rect(second), |ui| {
            ui.set_clip_rect(second);
            pane(ui, Pane::End);
        });
        let lit = response.hovered() || response.dragged() || response.has_focus();
        paint_divider(ui, x, rect, lit, palette);
        if lit {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        *ratio != clamped
    }

    /// The share the person asks for this frame, before the clamp: by dragging the divider, by
    /// a double click or Enter for the first share, or by the arrow, Home and End keys.
    fn asked_share(&self, ui: &Ui, response: &egui::Response, rect: Rect, ratio: f32) -> Option<f32> {
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            return Some((pointer.x - rect.left()) / rect.width());
        }
        if response.double_clicked() {
            return Some(self.initial);
        }
        if !response.has_focus() {
            return None;
        }
        ui.input(|input| {
            let step = if input.modifiers.shift { 0.1 } else { 0.02 };
            let keys = [
                (Key::ArrowLeft, ratio - step),
                (Key::ArrowRight, ratio + step),
                (Key::Home, 0.0),
                (Key::End, 1.0),
                (Key::Enter, self.initial),
            ];
            keys.into_iter()
                .find(|(key, _)| input.key_pressed(*key))
                .map(|(_, share)| share)
        })
    }
}

/// The divider line at `x`, thick and in `accent` while it is in use, with its grip.
fn paint_divider(ui: &Ui, x: f32, rect: Rect, lit: bool, palette: &Palette) {
    let (width, colour) = if lit {
        (BORDER_THICK, palette.accent)
    } else {
        (BORDER, palette.border)
    };
    let line = Rect::from_x_y_ranges(x - width / 2.0..=x + width / 2.0, rect.y_range());
    ui.painter().rect_filled(line, 0.0, colour);
    let grip = Rect::from_center_size(
        pos2(x, rect.center().y),
        vec2(du(dim::PAD_TIGHT), du(dim::ICON_LARGE)),
    );
    let mut y = grip.top();
    while y < grip.bottom() {
        let stripe = Rect::from_min_size(pos2(grip.left(), y), vec2(grip.width(), BORDER));
        ui.painter().rect_filled(stripe, 0.0, palette.border_strong);
        y += du(dim::GAP_MICRO);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    /// A split about 1000 points wide at 30%, its panes at least 25 DU, and its share.
    fn split() -> Harness<'static, f32> {
        let mut harness = Harness::builder().with_size(vec2(1000.0, 300.0)).build_ui_state(
            |ui, ratio: &mut f32| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let pane = SplitPane {
                    id: Id::unique("split"),
                    initial: 0.3,
                    min_size: 25.0,
                };
                pane.show(ui, ratio, &super::super::tokens::LIGHT, |_, _| {});
            },
            0.3,
        );
        harness.run();
        harness.get_by_role(Role::Splitter).focus();
        harness.run();
        harness
    }

    fn after(harness: &mut Harness<'_, f32>, modifiers: egui::Modifiers, key: Key) -> f32 {
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        (*harness.state() * 1000.0).round() / 1000.0
    }

    #[test]
    fn the_keyboard_moves_the_divider_within_the_minimum() {
        let mut harness = split();
        let none = egui::Modifiers::NONE;
        assert_eq!(after(&mut harness, none, Key::ArrowRight), 0.32);
        assert_eq!(after(&mut harness, egui::Modifiers::SHIFT, Key::ArrowLeft), 0.22);
        let lowest = after(&mut harness, none, Key::Home);
        assert!(
            (0.1..0.11).contains(&lowest),
            "25 DU of a pane a little under 1000 wide"
        );
        assert_eq!(
            after(&mut harness, none, Key::ArrowLeft),
            lowest,
            "no further than the minimum"
        );
        assert_eq!(after(&mut harness, none, Key::End), 1.0 - lowest);
        assert_eq!(
            after(&mut harness, none, Key::Enter),
            0.3,
            "Enter restores the first share"
        );
    }
}
