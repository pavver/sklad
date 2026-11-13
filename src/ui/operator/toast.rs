//! `Toast`: short messages in the corner that close by themselves.

use eframe::egui::{
    self, Align2, Context, Id, Order, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2,
};
use operator_ui_tokens::motion;

use super::button::IconButton;
use super::feedback::{Tone, overlay_shadow};
use super::icon::{self, paint_icon};
use super::motion::standard;
use super::state::{BORDER, BORDER_THICK};
use super::text::wrapped;
use super::tokens::{Palette, Type, dim, du, secs, text};

struct Toast {
    id: u64,
    tone: Tone,
    text: String,
    left: f32,
    born: f64,
    closing: Option<f64>,
}

impl Toast {
    /// A toast added between frames starts its clocks in the first frame that shows it.
    fn start_clocks(&mut self, now: f64) {
        if self.born.is_nan() {
            self.born = now;
        }
        if self.closing.is_some_and(f64::is_nan) {
            self.closing = Some(now);
        }
    }

    /// How much of the toast is there, `0` to `1`: it fades in after birth and out on closing.
    fn presence(&self, now: f64) -> f32 {
        let eased = |since: f64| standard(((now - since) as f32 / ENTER).min(1.0));
        eased(self.born) * (1.0 - self.closing.map_or(0.0, eased))
    }

    /// Counts the time of a toast nobody hovers down, and closes it when its time is up or its
    /// close button was pressed.
    fn tick(&mut self, closed: bool, hovered: bool, step: f32, now: f64) {
        if !hovered {
            self.left -= step;
        }
        let expired = self.left <= 0.0 && !hovered;
        if (closed || expired) && self.closing.is_none() {
            self.closing = Some(now);
        }
    }
}

/// `ToastHost`: the stack of toasts at the bottom centre of the window.
#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
    next: u64,
    last_frame: Option<f64>,
}

const ENTER: f32 = 0.15;

impl Toasts {
    pub fn message(&mut self, tone: Tone, text: String) {
        let duration = match tone {
            Tone::Error => motion::TOAST_ERROR_DURATION_MS,
            _ => motion::TOAST_DURATION_MS,
        };
        self.items.push(Toast {
            id: self.next,
            tone,
            text,
            left: secs(duration),
            born: f64::NAN,
            closing: None,
        });
        self.next += 1;
        while self.items.iter().filter(|toast| toast.closing.is_none()).count() > 3 {
            if let Some(oldest) = self.items.iter_mut().find(|toast| toast.closing.is_none()) {
                oldest.closing = Some(f64::NAN);
            }
        }
    }

    pub fn show(&mut self, ctx: &Context, close_label: &str, palette: &Palette) {
        let now = ctx.input(|input| input.time);
        let step = self.last_frame.map_or(0.0, |last| (now - last) as f32);
        self.last_frame = Some(now);
        self.items.retain(|toast| {
            toast
                .closing
                .is_none_or(|at| at.is_nan() || now - at < f64::from(ENTER))
        });
        if self.items.is_empty() {
            return;
        }
        let screen = ctx.content_rect();
        let width = du(105.0).min(screen.width() - du(8.0));
        let mut bottom = screen.bottom() - du(dim::MARGIN_SCREEN);
        for toast in self.items.iter_mut().rev() {
            toast.start_clocks(now);
            let presence = toast.presence(now);
            let shift = du(dim::GUTTER) * (1.0 - presence);
            let area = egui::Area::new(Id::unique(("toast", toast.id)))
                .order(Order::Foreground)
                .pivot(Align2::CENTER_BOTTOM)
                .fixed_pos(pos2(screen.center().x, bottom + shift))
                .show(ctx, |ui| {
                    ui.set_opacity(presence);
                    paint_toast(ui, toast, width, close_label, palette)
                });
            let (closed, height) = area.inner;
            toast.tick(closed, area.response.contains_pointer(), step, now);
            bottom -= height + du(dim::GUTTER);
        }
        ctx.request_repaint();
    }
}

/// Draws one toast. Returns whether its close button was pressed, and its height.
fn paint_toast(ui: &mut Ui, toast: &Toast, width: f32, close_label: &str, palette: &Palette) -> (bool, f32) {
    let colour = toast.tone.colour(palette);
    let icon_size = du(8.0);
    let close = du(8.0);
    let start = du(dim::PAD_CARD);
    let pad = du(dim::PAD_TIGHT);
    let text_width = width - start - icon_size - du(dim::GUTTER) - close - 2.0 * pad;
    let message = wrapped(
        ui,
        text(&toast.text, Type::Body, palette.text_primary),
        text_width,
    );
    let height = (message.size().y + 2.0 * pad).max(du(dim::ROW_CONTROL));
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let role = match toast.tone {
        Tone::Error => egui::Role::Alert,
        _ => egui::Role::Status,
    };
    response.widget_info(|| WidgetInfo::labeled(role, true, &toast.text));
    ui.painter().add(overlay_shadow().as_shape(rect, 0.0));
    ui.painter().rect_filled(rect, 0.0, palette.surface);
    ui.painter()
        .rect_stroke(rect, 0.0, Stroke::new(BORDER, palette.border), StrokeKind::Inside);
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, vec2(BORDER_THICK, height)),
        0.0,
        colour,
    );
    let icon_centre = pos2(rect.left() + start + icon_size / 2.0, rect.center().y);
    paint_icon(ui, toast.tone.icon(), icon_centre, icon_size, colour);
    let text_left = rect.left() + start + icon_size + du(dim::GUTTER);
    let text_top = rect.center().y - message.size().y / 2.0;
    ui.painter()
        .galley(pos2(text_left, text_top).round(), message, palette.text_primary);
    let button = Rect::from_min_size(
        pos2(rect.right() - pad - close, rect.center().y - close / 2.0),
        vec2(close, close),
    );
    let closed = ui
        .scope_builder(egui::UiBuilder::new().max_rect(button), |ui| {
            IconButton::new(icon::CLOSE, close_label)
                .show(ui, palette)
                .clicked()
        })
        .inner;
    (closed, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toast(left: f32) -> Toast {
        Toast {
            id: 0,
            tone: Tone::Success,
            text: String::new(),
            left,
            born: f64::NAN,
            closing: None,
        }
    }

    #[test]
    fn a_toast_fades_in_after_birth_and_out_on_closing() {
        let mut shown = toast(5.0);
        shown.start_clocks(10.0);
        assert_eq!(shown.presence(10.0), 0.0);
        assert_eq!(shown.presence(10.0 + f64::from(ENTER)), 1.0);
        shown.closing = Some(f64::NAN);
        shown.start_clocks(20.0);
        assert_eq!(
            shown.closing,
            Some(20.0),
            "a close asked between frames starts now"
        );
        assert_eq!(shown.presence(20.0), 1.0);
        assert_eq!(shown.presence(20.0 + f64::from(ENTER)), 0.0);
    }

    #[test]
    fn a_toast_waits_while_hovered_and_closes_when_its_time_is_up() {
        let mut waiting = toast(1.0);
        waiting.tick(false, true, 2.0, 3.0);
        assert_eq!(
            (waiting.left, waiting.closing),
            (1.0, None),
            "hovered, it keeps its time"
        );
        waiting.tick(false, false, 0.5, 4.0);
        assert_eq!(waiting.closing, None);
        waiting.tick(false, false, 0.5, 5.0);
        assert_eq!(waiting.closing, Some(5.0), "time is up");
        waiting.tick(true, false, 0.5, 6.0);
        assert_eq!(waiting.closing, Some(5.0), "closing once is enough");

        let mut pressed = toast(9.0);
        pressed.tick(true, true, 0.0, 7.0);
        assert_eq!(pressed.closing, Some(7.0), "the close button closes it at once");
    }
}
