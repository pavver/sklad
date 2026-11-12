//! `ActionButton` and `IconButton` (`docs/components/action-button.md`, `icon-button.md`).

use eframe::egui::{self, Color32, CursorIcon, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};
use operator_ui_tokens::motion;

use super::icon::{self, Icon, paint_icon};
use super::state::{
    ControlState, Look, Outcome, Outline, hover_amount, paint_border, paint_focus_ring,
    paint_pending_indicator, paint_result_background, paint_ripple, press_amount, result_colour,
};
use super::text::{galley, paint_galley};
use super::tokens::{MEDIUM, Palette, Type, dim, du, mix, secs, tint, weighted};

/// Backgrounds of a control at rest, under the pointer and pressed.
#[derive(Clone, Copy)]
pub struct Fills {
    pub rest: Color32,
    pub hover: Color32,
    pub pressed: Color32,
}

impl Fills {
    pub fn plain(palette: &Palette) -> Self {
        Self {
            rest: palette.surface,
            hover: palette.hover,
            pressed: palette.pressed,
        }
    }

    pub fn danger(palette: &Palette) -> Self {
        Self {
            rest: tint(palette.error, 0.15),
            hover: tint(palette.error, 0.22),
            pressed: tint(palette.error, 0.30),
        }
    }
}

/// Paints the body of an interactive control: fill, ripple and result background. Returns the
/// state its border should show once the content is drawn.
pub fn paint_body(
    ui: &Ui,
    rect: Rect,
    response: &Response,
    fills: Fills,
    look: &Look,
    enabled: bool,
    palette: &Palette,
) -> ControlState {
    let painter = ui.painter();
    if !enabled {
        painter.rect_filled(rect, 0.0, palette.disabled_surface);
        return ControlState::Disabled;
    }
    let calm = !look.moves();
    let hover = if calm { hover_amount(ui, response) } else { 0.0 };
    let press = if look.is_pending() {
        0.0
    } else {
        press_amount(ui, response)
    };
    let fill = mix(mix(fills.rest, fills.hover, hover), fills.pressed, press);
    painter.rect_filled(rect, 0.0, fill);
    paint_result_background(ui, rect, look, palette);
    paint_ripple(ui, response, rect, !look.is_pending(), palette);
    if look.is_pending() {
        ControlState::Pending
    } else if press > 0.5 || (calm && hover > 0.5) {
        ControlState::Pressed
    } else {
        look.state()
    }
}

/// The colour of content after a result: the result colour, fading back to `base`.
pub fn content_colour(base: Color32, look: &Look, palette: &Palette) -> Color32 {
    look.result.map_or(base, |outcome| {
        mix(base, result_colour(outcome, palette), look.result_alpha)
    })
}

/// Whether the result icon stands in place of the content: the first 500 ms of a result.
fn flashing_icon(look: &Look) -> Option<Outcome> {
    let outcome = look.result?;
    (look.result_age < secs(motion::LOCAL_FLASH_ICON_MS)).then_some(outcome)
}

fn result_icon(outcome: Outcome) -> Icon {
    match outcome {
        Outcome::Success => icon::CHECK,
        Outcome::Error => icon::CLOSE,
    }
}

fn sense(enabled: bool, look: &Look, locked: bool) -> Sense {
    if enabled && !look.is_pending() && !locked {
        Sense::click()
    } else {
        Sense::hover()
    }
}

fn pointing(response: Response) -> Response {
    if response.sense.senses_click() {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// A rectangular button that runs an action and shows how it ended.
pub struct ActionButton<'a> {
    text: &'a str,
    icon: Option<Icon>,
    look: Look,
    enabled: bool,
    dangerous: bool,
}

impl<'a> ActionButton<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            icon: None,
            look: Look::IDLE,
            enabled: true,
            dangerous: false,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The state of the request this button runs.
    pub fn look(mut self, look: Look) -> Self {
        self.look = look;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn dangerous(mut self, dangerous: bool) -> Self {
        self.dangerous = dangerous;
        self
    }

    pub fn show(self, ui: &mut Ui, palette: &Palette) -> Response {
        let height = du(dim::ROW_STANDARD);
        let label = galley(ui, weighted(self.text, Type::Body, MEDIUM, Color32::PLACEHOLDER));
        let icon_size = du(dim::ICON_SMALL);
        let gap = du(dim::GUTTER);
        let icon_width = self.icon.map_or(0.0, |_| icon_size + gap);
        let content = icon_width + label.size().x;
        let width = (content + 2.0 * du(dim::MARGIN_SCREEN)).ceil();
        let locked = self.dangerous && self.look.result.is_some();
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, height), sense(self.enabled, &self.look, locked));
        let enabled = self.enabled;
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, enabled, self.text));
        if !ui.is_rect_visible(rect) {
            return pointing(response);
        }
        let fills = if self.dangerous {
            Fills::danger(palette)
        } else {
            Fills::plain(palette)
        };
        let state = paint_body(ui, rect, &response, fills, &self.look, enabled, palette);
        let base = match (enabled, self.dangerous) {
            (false, _) => palette.text_disabled,
            (true, true) => palette.error,
            (true, false) => palette.text_primary,
        };
        let colour = content_colour(base, &self.look, palette);

        if let Some(outcome) = flashing_icon(&self.look) {
            let colour = result_colour(outcome, palette);
            paint_icon(ui, result_icon(outcome), rect.center(), icon_size, colour);
        } else if self.look.spinner() > 0.0 {
            paint_pending_indicator(ui, rect.center(), icon_size, &self.look, palette);
        } else {
            let left = rect.center().x - content / 2.0;
            if let Some(icon) = self.icon {
                let centre = pos2(left + icon_size / 2.0, rect.center().y);
                paint_icon(ui, icon, centre, icon_size, colour);
            }
            let text_rect =
                Rect::from_min_size(pos2(left + icon_width, rect.top()), vec2(label.size().x, height));
            paint_galley(ui, text_rect, label, colour);
        }

        paint_border(ui, rect, Outline::of(state, palette), &self.look, palette);
        paint_focus_ring(ui, &response, rect, palette);
        pointing(response)
    }
}

/// A square button with one icon. Its label is the accessible name and the tooltip.
pub struct IconButton<'a> {
    icon: Icon,
    label: &'a str,
    size: f32,
    icon_size: f32,
    look: Look,
    enabled: bool,
    selected: bool,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: Icon, label: &'a str) -> Self {
        Self {
            icon,
            label,
            size: dim::ROW_COMPACT,
            icon_size: dim::ICON_SMALL,
            look: Look::IDLE,
            enabled: true,
            selected: false,
        }
    }

    /// Side in DU.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Icon side in DU.
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size;
        self
    }

    pub fn look(mut self, look: Look) -> Self {
        self.look = look;
        self
    }

    pub fn show(self, ui: &mut Ui, palette: &Palette) -> Response {
        let side = du(self.size);
        let (rect, response) =
            ui.allocate_exact_size(vec2(side, side), sense(self.enabled, &self.look, false));
        let enabled = self.enabled;
        let selected = self.selected;
        response.widget_info(|| WidgetInfo::selected(egui::Role::Button, enabled, selected, self.label));
        if ui.is_rect_visible(rect) {
            self.paint(ui, rect, &response, palette);
        }
        pointing(response).on_hover_text(self.label)
    }

    fn paint(&self, ui: &Ui, rect: Rect, response: &Response, palette: &Palette) {
        let mut fills = Fills::plain(palette);
        if self.selected {
            fills.rest = palette.selection;
        }
        let mut state = paint_body(ui, rect, response, fills, &self.look, self.enabled, palette);
        if self.selected && state == ControlState::Idle {
            state = ControlState::Selected;
        }
        let icon_size = du(self.icon_size);
        let base = if self.enabled {
            palette.icon
        } else {
            palette.icon_disabled
        };
        if let Some(outcome) = flashing_icon(&self.look) {
            let colour = result_colour(outcome, palette);
            paint_icon(ui, result_icon(outcome), rect.center(), icon_size, colour);
        } else if self.look.spinner() > 0.0 {
            paint_pending_indicator(ui, rect.center(), icon_size, &self.look, palette);
        } else {
            let colour = content_colour(base, &self.look, palette);
            paint_icon(ui, self.icon, rect.center(), icon_size, colour);
        }
        paint_border(ui, rect, Outline::of(state, palette), &self.look, palette);
        paint_focus_ring(ui, response, rect, palette);
    }
}
