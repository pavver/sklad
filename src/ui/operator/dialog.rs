//! `Dialog` with a confirm action (`docs/components/dialog.md`): the modal shell, its header, a
//! scrolling body and the action row. The caller runs the request and reports its life as a `Look`.

use eframe::egui::{
    self, Align, Color32, Context, Frame, Id, Layout, Modal, Rect, ScrollArea, Sense, Stroke, Ui, UiBuilder,
    pos2, vec2,
};
use eframe::emath::TSTransform;
use operator_ui_tokens::motion;

use super::button::{ActionButton, IconButton};
use super::icon::{self, Icon};
use super::motion::standard;
use super::state::{BORDER, Look, Outcome, Outline, paint_border};
use super::text::wrapped;
use super::tokens::{Palette, Type, dim, du, secs, text};

/// What the person asked of the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    Confirm,
    Close,
}

pub struct Dialog<'a> {
    pub id: Id,
    pub title: &'a str,
    pub confirm_text: &'a str,
    pub cancel_text: &'a str,
    pub close_label: &'a str,
    /// Without a confirm action the dialog has a close button instead of Cancel and confirm.
    pub has_confirm: bool,
    pub confirm_icon: Option<Icon>,
    pub confirm_enabled: bool,
    pub destructive: bool,
    /// The life of the confirm request.
    pub look: Look,
    /// When the dialog opened, for its entry.
    pub opened: f64,
}

/// How long a result is shown on the dialog: success closes after it, error fades after it.
pub fn dialog_hold() -> f32 {
    0.5
}

impl Dialog<'_> {
    /// Confirm and Cancel stay locked while the request runs, during an error, and `300 ms` after.
    fn locked(&self) -> bool {
        self.look.is_pending()
            || (self.look.result == Some(Outcome::Error) && self.look.result_age < dialog_hold() + 0.3)
            || self.look.result == Some(Outcome::Success)
    }

    pub fn show(
        &self,
        ctx: &Context,
        palette: &Palette,
        body: impl FnOnce(&mut Ui),
        actions: impl FnOnce(&mut Ui),
    ) -> Option<Asked> {
        let now = ctx.input(|input| input.time);
        let entry = standard(((now - self.opened) as f32 / secs(motion::DIALOG_ENTER_MS)).min(1.0));
        if entry < 1.0 {
            ctx.request_repaint();
        }
        let screen = ctx.content_rect();
        let width = (screen.width() * 0.9)
            .clamp(du(75.0), du(140.0))
            .min(screen.width());
        let max_height = screen.height() - du(8.0);
        let scrim = Color32::from_black_alpha((0.55 * 255.0 * entry) as u8);
        let mut asked = None;
        let modal = Modal::new(self.id)
            .frame(Frame::NONE)
            .backdrop_color(scrim)
            .show(ctx, |ui| {
                ui.set_opacity(entry);
                let (rect, _) = ui.allocate_exact_size(vec2(width, 0.0), Sense::hover());
                let top = rect.top();
                let background = ui.painter().add(egui::Shape::Noop);
                asked = self.header(ui, width, palette);
                let header_bottom = ui.min_rect().bottom();
                let actions_height = du(dim::ROW_STANDARD) + 2.0 * du(dim::PAD_CARD);
                let body_limit = (max_height - (header_bottom - top) - actions_height).max(du(20.0));
                ui.scope_builder(UiBuilder::new(), |ui| {
                    ScrollArea::vertical()
                        .id_salt(self.id.with("body"))
                        .max_height(body_limit)
                        .show(ui, |ui| {
                            Frame::new().inner_margin(du(dim::MARGIN_SCREEN)).show(ui, |ui| {
                                ui.set_width(width - 2.0 * du(dim::MARGIN_SCREEN));
                                ui.spacing_mut().item_spacing = vec2(du(dim::PAD_CARD), du(dim::PAD_CARD));
                                body(ui);
                            });
                        });
                });
                if self.has_confirm
                    && let Some(choice) = self.actions(ui, width, palette, actions)
                {
                    asked = Some(choice);
                }
                let whole = Rect::from_min_max(
                    pos2(rect.left(), top),
                    pos2(rect.left() + width, ui.min_rect().bottom()),
                );
                ui.painter()
                    .set(background, egui::Shape::rect_filled(whole, 0.0, palette.surface));
                paint_border(
                    ui,
                    whole,
                    Outline::of(self.look.state(), palette),
                    &self.look,
                    palette,
                );
                whole
            });
        let whole = modal.inner;
        let layer = modal.response.layer_id;
        let scale = 0.95 + 0.05 * entry;
        let centre = whole.center().to_vec2();
        let transform = TSTransform::from_translation(centre)
            * TSTransform::from_scaling(scale)
            * TSTransform::from_translation(-centre);
        ctx.set_transform_layer(layer, transform);
        if modal.should_close() && !self.locked() && asked.is_none() {
            asked = Some(Asked::Close);
        }
        asked
    }

    fn header(&self, ui: &mut Ui, width: f32, palette: &Palette) -> Option<Asked> {
        let pad = du(dim::GUTTER);
        let start = du(dim::MARGIN_SCREEN);
        let close = du(dim::ROW_COMPACT);
        let room = width - start - pad - if self.has_confirm { 0.0 } else { close + pad };
        let title = wrapped(
            ui,
            text(self.title, Type::ScreenTitle, palette.text_primary),
            room,
        );
        let height = title.size().y.max(close) + 2.0 * pad;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let title_top = rect.center().y - title.size().y / 2.0;
        ui.painter().galley(
            pos2(rect.left() + start, title_top).round(),
            title,
            palette.text_primary,
        );
        let mut asked = None;
        if !self.has_confirm {
            let button = Rect::from_min_size(
                pos2(rect.right() - pad - close, rect.center().y - close / 2.0),
                vec2(close, close),
            );
            let clicked = ui
                .scope_builder(UiBuilder::new().max_rect(button), |ui| {
                    IconButton::new(icon::CLOSE, self.close_label)
                        .show(ui, palette)
                        .clicked()
                })
                .inner;
            if clicked {
                asked = Some(Asked::Close);
            }
        }
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - BORDER / 2.0,
            Stroke::new(BORDER, palette.divider),
        );
        asked
    }

    fn actions(
        &self,
        ui: &mut Ui,
        width: f32,
        palette: &Palette,
        extra: impl FnOnce(&mut Ui),
    ) -> Option<Asked> {
        let height = du(dim::ROW_STANDARD) + 2.0 * du(dim::PAD_CARD);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter().hline(
            rect.x_range(),
            rect.top() + BORDER / 2.0,
            Stroke::new(BORDER, palette.divider),
        );
        let inner = rect.shrink2(vec2(du(dim::MARGIN_SCREEN), du(dim::PAD_CARD)));
        let locked = self.locked();
        let mut asked = None;
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing = vec2(du(dim::GUTTER), 0.0);
                let mut confirm = ActionButton::new(self.confirm_text)
                    .dangerous(self.destructive)
                    .enabled(self.confirm_enabled)
                    .look(self.look);
                if let Some(glyph) = self.confirm_icon {
                    confirm = confirm.icon(glyph);
                }
                if confirm.show(ui, palette).clicked() && !locked {
                    asked = Some(Asked::Confirm);
                }
                let cancel = ActionButton::new(self.cancel_text)
                    .enabled(!locked)
                    .show(ui, palette);
                if cancel.clicked() {
                    asked = Some(Asked::Close);
                }
                extra(ui);
            },
        );
        asked
    }
}
