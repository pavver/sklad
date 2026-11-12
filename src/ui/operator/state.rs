//! The state machine and feedback language of `docs/foundations/states.md`: control states, the
//! async life of a request, the state border, the press ripple, the spinner and the focus ring.

use std::f32::consts::TAU;

use eframe::egui::{self, Color32, Id, Pos2, Rect, Response, Stroke, StrokeKind, Ui, pos2};
use operator_ui_tokens::motion;

use super::motion::{smooth, standard};
use super::tokens::{Palette, dim, du, secs, tint};

pub const BORDER: f32 = du(dim::BORDER);
pub const BORDER_THICK: f32 = du(dim::BORDER_THICK);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlState {
    Idle,
    Pressed,
    Focused,
    Selected,
    Pending,
    Success,
    Error,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Error,
}

/// What a control shows of its request at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// Seconds the request has been running.
    pub pending: Option<f32>,
    /// How the last request ended, while it is still on screen.
    pub result: Option<Outcome>,
    /// Seconds since the result arrived.
    pub result_age: f32,
    /// 1 while the result is held, then down to 0 over the exit fade.
    pub result_alpha: f32,
}

impl Look {
    pub const IDLE: Look = Look {
        pending: None,
        result: None,
        result_age: 0.0,
        result_alpha: 0.0,
    };

    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// The result is held, not fading.
    pub fn holding(&self) -> bool {
        self.result.is_some() && self.result_alpha >= 1.0
    }

    pub fn moves(&self) -> bool {
        self.pending.is_some() || self.result.is_some()
    }

    /// Opacity of the delayed spinner: nothing for 700 ms, then a 200 ms fade in.
    pub fn spinner(&self) -> f32 {
        let delay = secs(motion::PENDING_INDICATOR_DELAY_MS);
        let fade = secs(motion::PENDING_INDICATOR_FADE_MS);
        self.pending
            .map_or(0.0, |elapsed| ((elapsed - delay) / fade).clamp(0.0, 1.0))
    }

    /// The state this look stands for, before hover and focus.
    pub fn state(&self) -> ControlState {
        match (self.pending, self.result) {
            (Some(_), _) => ControlState::Pending,
            (None, Some(Outcome::Success)) if self.holding() => ControlState::Success,
            (None, Some(Outcome::Error)) if self.holding() => ControlState::Error,
            _ => ControlState::Idle,
        }
    }
}

/// The async life of one control, kept by whoever runs the request.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Activity {
    pending_since: Option<f64>,
    result: Option<(Outcome, f64)>,
}

/// How long a component keeps its own result before the exit fade.
pub fn result_hold() -> f32 {
    secs(motion::RESULT_HOLD_MS)
}

impl Activity {
    /// A request starts. A repeated start does not restart the spinner delay.
    pub fn start(&mut self, now: f64) {
        if self.pending_since.is_none() {
            self.pending_since = Some(now);
        }
        self.result = None;
    }

    pub fn finish(&mut self, outcome: Outcome, now: f64) {
        self.pending_since = None;
        self.result = Some((outcome, now));
    }

    /// The look at `now`, holding a result for `hold` seconds before it fades.
    pub fn look(&self, now: f64, hold: f32) -> Look {
        if let Some(since) = self.pending_since {
            return Look {
                pending: Some((now - since) as f32),
                ..Look::IDLE
            };
        }
        let Some((outcome, at)) = self.result else {
            return Look::IDLE;
        };
        let age = (now - at) as f32;
        let fade = secs(motion::RESULT_FADE_MS);
        let alpha = 1.0 - smooth((age - hold) / fade);
        if alpha <= 0.0 {
            return Look::IDLE;
        }
        Look {
            pending: None,
            result: Some(outcome),
            result_age: age,
            result_alpha: alpha,
        }
    }

    /// An activity a widget keeps for itself between frames.
    pub fn load(ctx: &egui::Context, id: Id) -> Self {
        ctx.data(|data| data.get_temp(id)).unwrap_or_default()
    }

    pub fn store(self, ctx: &egui::Context, id: Id) {
        ctx.data_mut(|data| data.insert_temp(id, self));
    }
}

/// The base outline of a state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub colour: Color32,
    pub width: f32,
}

impl Outline {
    pub fn of(state: ControlState, palette: &Palette) -> Self {
        let (colour, width) = match state {
            ControlState::Pressed => (palette.border_strong, BORDER),
            ControlState::Focused | ControlState::Selected => (palette.accent, BORDER * 1.5),
            ControlState::Disabled => (palette.disabled_border, BORDER),
            _ => (palette.border, BORDER),
        };
        Self { colour, width }
    }
}

pub fn result_colour(outcome: Outcome, palette: &Palette) -> Color32 {
    match outcome {
        Outcome::Success => palette.success,
        Outcome::Error => palette.error,
    }
}

/// The result colour at 15% on its own layer, for components that tint their background.
pub fn paint_result_background(ui: &Ui, rect: Rect, look: &Look, palette: &Palette) {
    if let Some(outcome) = look.result {
        let colour = tint(result_colour(outcome, palette), 0.15 * look.result_alpha);
        ui.painter().rect_filled(rect, 0.0, colour);
    }
}

/// The state border on top of the content: base outline, result pulse, pending segment.
pub fn paint_border(ui: &Ui, rect: Rect, base: Outline, look: &Look, palette: &Palette) {
    let painter = ui.painter();
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(base.width, base.colour),
        StrokeKind::Inside,
    );
    if let Some(outcome) = look.result {
        let half = secs(motion::RESULT_PULSE_HALF_MS);
        let phase = look.result_age / half;
        let swing = if (phase as u32).is_multiple_of(2) {
            phase.fract()
        } else {
            1.0 - phase.fract()
        };
        let alpha = (0.55 + 0.45 * standard(swing)) * look.result_alpha;
        let stroke = Stroke::new(BORDER * 2.0, tint(result_colour(outcome, palette), alpha));
        painter.rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
    }
    if let Some(elapsed) = look.pending {
        let width = BORDER * 1.5;
        let inset = rect.shrink(width / 2.0);
        let perimeter = 2.0 * (inset.width() + inset.height());
        let start = (elapsed / secs(motion::PENDING_LAP_MS)).fract() * perimeter;
        let points = along_outline(inset, start, perimeter * 0.25);
        painter.add(egui::Shape::line(points, Stroke::new(width, palette.pending)));
    }
    if look.moves() {
        ui.ctx().request_repaint();
    }
}

/// Points of the outline of `rect`, clockwise from the top left corner, from `start` for `length`.
fn along_outline(rect: Rect, start: f32, length: f32) -> Vec<Pos2> {
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    let edges: Vec<(Pos2, Pos2)> = (0..4).map(|i| (corners[i], corners[(i + 1) % 4])).collect();
    let perimeter = 2.0 * (rect.width() + rect.height());
    if perimeter <= 0.0 {
        return Vec::new();
    }
    let at = |distance: f32| {
        let mut left = distance.rem_euclid(perimeter);
        for (from, to) in &edges {
            let span = from.distance(*to);
            if left <= span {
                return *from + (*to - *from) * (left / span.max(f32::EPSILON));
            }
            left -= span;
        }
        corners[0]
    };
    let mut points = vec![at(start)];
    let mut walked = 0.0;
    let mut cursor = start.rem_euclid(perimeter);
    let mut offsets = [
        0.0,
        rect.width(),
        rect.width() + rect.height(),
        2.0 * rect.width() + rect.height(),
    ];
    offsets.iter_mut().for_each(|offset| {
        if *offset <= cursor {
            *offset += perimeter;
        }
    });
    offsets.sort_by(f32::total_cmp);
    for corner in offsets {
        let step = corner - cursor;
        if walked + step >= length {
            break;
        }
        walked += step;
        cursor = corner;
        points.push(at(corner));
    }
    points.push(at(start + length));
    points
}

#[derive(Clone, Copy, Debug)]
struct Ripple {
    origin: Pos2,
    start: f64,
    released: Option<f64>,
}

/// The bounded press ripple, spreading from the pointer inside `rect`.
pub fn paint_ripple(ui: &Ui, response: &Response, rect: Rect, enabled: bool, palette: &Palette) {
    let id = response.id.with("ripple");
    let ctx = ui.ctx();
    let now = ui.input(|input| input.time);
    let down = response.is_pointer_button_down_on();
    let mut ripple: Option<Ripple> = ctx.data(|data| data.get_temp(id));
    let pressed_now = ui.input(|input| input.pointer.any_pressed());
    if enabled
        && down
        && pressed_now
        && let Some(origin) = ui.input(|input| input.pointer.press_origin())
    {
        ripple = Some(Ripple {
            origin,
            start: now,
            released: None,
        });
    }
    let Some(mut current) = ripple else {
        return;
    };
    if current.released.is_none() && !down {
        current.released = Some(now);
    }
    let grow = 0.33;
    let growth = standard(((now - current.start) as f32 / grow).min(1.0));
    let mut alpha = if palette.dark { 0.24 } else { 0.16 };
    if let Some(released) = current.released {
        let fade_from = released.max(current.start + f64::from(grow));
        alpha *= 1.0 - ((now - fade_from) as f32 / 0.22).clamp(0.0, 1.0);
    }
    if alpha <= 0.0 {
        ctx.data_mut(|data| data.remove::<Ripple>(id));
        return;
    }
    let reach = [
        rect.left_top(),
        rect.right_top(),
        rect.left_bottom(),
        rect.right_bottom(),
    ]
    .iter()
    .map(|corner| corner.distance(current.origin))
    .fold(0.0, f32::max);
    ui.painter().with_clip_rect(rect).circle_filled(
        current.origin,
        reach * growth,
        tint(palette.ripple, alpha),
    );
    ctx.data_mut(|data| data.insert_temp(id, current));
    ctx.request_repaint();
}

/// How far the pointer has come over a control, 0 to 1, over `state_color`.
pub fn hover_amount(ui: &Ui, response: &Response) -> f32 {
    ui.ctx().animate_bool_with_time_and_easing(
        response.id.with("hover"),
        response.hovered() && response.enabled(),
        secs(motion::STATE_COLOR_MS),
        standard,
    )
}

/// How far a press has come, 0 to 1.
pub fn press_amount(ui: &Ui, response: &Response) -> f32 {
    ui.ctx().animate_bool_with_time_and_easing(
        response.id.with("press"),
        response.is_pointer_button_down_on() && response.enabled(),
        secs(motion::STATE_COLOR_MS) / 2.0,
        standard,
    )
}

/// The `0.5 DU` accent ring outside a control that has keyboard focus.
pub fn paint_focus_ring(ui: &Ui, response: &Response, rect: Rect, palette: &Palette) {
    if response.has_focus() {
        let ring = rect.expand(du(dim::GAP_MICRO));
        let stroke = Stroke::new(BORDER_THICK, palette.accent);
        ui.painter().rect_stroke(ring, 0.0, stroke, StrokeKind::Outside);
    }
}

/// `ProgressIndicator`: an arc of 270° turning once per `spinner_lap`, with round caps.
pub fn paint_spinner(ui: &Ui, centre: Pos2, diameter: f32, width: f32, colour: Color32) {
    let time = ui.input(|input| input.time) as f32;
    let turn = (time / secs(motion::SPINNER_LAP_MS)).fract() * TAU;
    let radius = (diameter - width) / 2.0;
    let steps = 36;
    let points: Vec<Pos2> = (0..=steps)
        .map(|step| {
            let angle = turn + 0.75 * TAU * step as f32 / steps as f32;
            pos2(centre.x + radius * angle.cos(), centre.y + radius * angle.sin())
        })
        .collect();
    let painter = ui.painter();
    if let (Some(first), Some(last)) = (points.first(), points.last()) {
        painter.circle_filled(*first, width / 2.0, colour);
        painter.circle_filled(*last, width / 2.0, colour);
    }
    painter.add(egui::Shape::line(points, Stroke::new(width, colour)));
    ui.ctx().request_repaint();
}

/// `PendingIndicator`: the spinner inside an async control, in `pending`, after the delay.
pub fn paint_pending_indicator(ui: &Ui, centre: Pos2, size: f32, look: &Look, palette: &Palette) {
    let alpha = look.spinner();
    if alpha > 0.0 {
        let width = du(dim::SPINNER_STROKE);
        paint_spinner(ui, centre, size, width, tint(palette.pending, alpha));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_is_held_then_fades_and_is_gone() {
        let mut activity = Activity::default();
        activity.start(0.0);
        assert_eq!(activity.look(0.3, 0.8).state(), ControlState::Pending);
        activity.finish(Outcome::Error, 1.0);
        assert_eq!(activity.look(1.5, 0.8).state(), ControlState::Error);
        let fading = activity.look(2.0, 0.8);
        assert!(fading.result_alpha > 0.0 && fading.result_alpha < 1.0);
        assert_eq!(activity.look(2.4, 0.8), Look::IDLE);
    }

    #[test]
    fn the_spinner_waits_for_a_slow_request() {
        let mut activity = Activity::default();
        activity.start(0.0);
        assert_eq!(activity.look(0.5, 0.8).spinner(), 0.0);
        assert_eq!(activity.look(1.0, 0.8).spinner(), 1.0);
        activity.start(0.9);
        assert_eq!(
            activity.look(1.0, 0.8).spinner(),
            1.0,
            "a repeated start keeps the delay"
        );
    }

    #[test]
    fn the_pending_segment_turns_the_corner() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        let points = along_outline(rect, 5.0, 10.0);
        assert_eq!(points.first(), Some(&pos2(5.0, 0.0)));
        assert!(points.contains(&pos2(10.0, 0.0)));
        assert_eq!(points.last(), Some(&pos2(10.0, 5.0)));
    }
}
