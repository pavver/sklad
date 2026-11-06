//! The removal dialog: shows exactly what will happen, then does it and shows the output.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Frame, Id, Layout, Margin, Modal, RichText, ScrollArea, Stroke, Ui, vec2};

use super::format;
use super::glyph;
use super::theme::{
    BORDER, MARGIN_SCREEN, PAD_CARD, PAD_TIGHT, Palette, TERMINAL_BACKGROUND, TERMINAL_FOREGROUND, Type,
    Weight, clamp_width, du, styled, weighted,
};
use super::widgets::{ControlState, Tone, action_button, copy_button, segmented, state_border};
use crate::i18n::Text;
use crate::model::{Access, Item, Removal, Usage};
use crate::plan::{Mode, PacmanPlan};
use crate::remove::{Backend, Event, Outcome, Spec, is_irreversible, other_spec, pacman_spec};
use crate::scan::Catalog;

/// The success state stays on screen this long, then the dialog closes.
const SUCCESS_HOLD: Duration = Duration::from_millis(500);

type Preview = Result<Vec<String>, String>;

enum Phase {
    /// Pacman is asked whether it accepts the list.
    Checking(Receiver<Preview>),
    /// The person decides. `spec` is empty when there is nothing that can be run.
    Ready {
        spec: Option<Spec>,
        refusal: Option<String>,
    },
    Running(Receiver<Event>),
    Succeeded(Instant),
    Failed(String),
}

pub enum Exit {
    Cancelled,
    Removed(String),
}

pub struct Dialog {
    item: Item,
    catalog: Arc<Catalog>,
    backend: Arc<dyn Backend>,
    text: &'static Text,
    mode: Mode,
    plan: Option<PacmanPlan>,
    phase: Phase,
    confirmed_risk: bool,
    note: Option<String>,
    log: Vec<String>,
}

/// A block in the terminal palette, which never derives from the interface colours.
pub fn terminal(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(TERMINAL_BACKGROUND)
        .inner_margin(Margin::same(du(2.0) as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui);
        });
}

fn wrapped(ui: &mut Ui, text: RichText) {
    ui.add(egui::Label::new(text).wrap());
}

fn warning(ui: &mut Ui, text: impl Into<String>, colour: egui::Color32) {
    Frame::new()
        .stroke(Stroke::new(BORDER, colour))
        .inner_margin(Margin::same(du(2.0) as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let line = format!("{} {}", glyph::WARNING, text.into());
            wrapped(ui, styled(line, Type::Body, colour));
        });
}

impl Dialog {
    pub fn open(item: Item, catalog: Arc<Catalog>, backend: Arc<dyn Backend>, text: &'static Text) -> Self {
        let mut dialog = Self {
            item,
            catalog,
            backend,
            text,
            mode: Mode::Keep,
            plan: None,
            phase: Phase::Ready {
                spec: None,
                refusal: None,
            },
            confirmed_risk: false,
            note: None,
            log: Vec::new(),
        };
        dialog.prepare();
        dialog
    }

    fn prepare(&mut self) {
        self.confirmed_risk = false;
        if self.item.removal != Removal::Pacman {
            let spec = other_spec(&self.item.removal);
            let refusal = spec.is_none().then(|| self.text.path_too_close.to_owned());
            self.phase = Phase::Ready { spec, refusal };
            return;
        }
        self.plan = self.catalog.db.plan(&self.item.package, self.mode);
        let Some(plan) = self.plan.as_ref().filter(|plan| !plan.is_blocked()) else {
            self.phase = Phase::Ready {
                spec: None,
                refusal: None,
            };
            return;
        };
        let (sender, receiver) = channel();
        let backend = Arc::clone(&self.backend);
        let names = plan.remove.clone();
        thread::spawn(move || {
            let _ = sender.send(backend.preview(&names));
        });
        self.phase = Phase::Checking(receiver);
    }

    /// Pacman must return the same list: any difference means the plan is not what would happen.
    fn review(&self, answer: Preview) -> Phase {
        let Some(plan) = &self.plan else {
            return Phase::Ready {
                spec: None,
                refusal: None,
            };
        };
        match answer {
            Ok(list) if list == plan.remove => Phase::Ready {
                spec: Some(pacman_spec(plan)),
                refusal: None,
            },
            Ok(_) => Phase::Ready {
                spec: None,
                refusal: Some(self.text.other_list_refused.to_owned()),
            },
            Err(message) => Phase::Ready {
                spec: None,
                refusal: Some((self.text.pacman_refused)(&message)),
            },
        }
    }

    fn poll(&mut self) {
        match &self.phase {
            Phase::Checking(receiver) => {
                if let Ok(answer) = receiver.try_recv() {
                    self.phase = self.review(answer);
                }
            }
            Phase::Running(receiver) => {
                let events: Vec<Event> = receiver.try_iter().collect();
                for event in events {
                    match event {
                        Event::Line(line) => self.log.push(line),
                        Event::Finished(Outcome::Done) => self.phase = Phase::Succeeded(Instant::now()),
                        Event::Finished(Outcome::Failed(failure)) => {
                            self.phase = Phase::Failed(self.text.failure(&failure));
                        }
                        Event::Finished(Outcome::Dismissed) => {
                            self.note = Some(self.text.no_password.to_owned());
                            self.prepare();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn needs_risk_confirmation(&self) -> bool {
        is_irreversible(&self.item) || self.plan.as_ref().is_some_and(|plan| !plan.critical.is_empty())
    }

    fn state(&self) -> ControlState {
        match self.phase {
            Phase::Checking(_) | Phase::Running(_) => ControlState::Pending,
            Phase::Succeeded(_) => ControlState::Success,
            Phase::Failed(_) => ControlState::Error,
            Phase::Ready { .. } => ControlState::Idle,
        }
    }

    fn pacman_section(&mut self, ui: &mut Ui, palette: &Palette) {
        let Some(plan) = self.plan.clone() else {
            return;
        };
        if plan.is_blocked() {
            let names = plan.blockers.join(", ");
            warning(ui, (self.text.needed_by_programs)(&names), palette.pending);
        }
        if plan.is_blocked() || self.mode == Mode::WithDependents {
            let selected = usize::from(self.mode == Mode::WithDependents);
            let labels = [self.text.keep_everything, self.text.remove_with_them];
            if let Some(choice) = segmented(ui, &labels, selected, palette) {
                self.mode = if choice == 1 {
                    Mode::WithDependents
                } else {
                    Mode::Keep
                };
                self.prepare();
                return;
            }
        }
        if plan.is_blocked() {
            return;
        }
        if !plan.metas.is_empty() {
            let metas = plan.metas.join(", ");
            let kept = plan.keep.len();
            let text = (self.text.held_by_meta)(&metas, kept);
            wrapped(ui, styled(text, Type::Body, palette.text_primary));
        }
        if !plan.critical.is_empty() {
            let names = plan.critical.join(", ");
            warning(ui, (self.text.critical_packages)(&names), palette.error);
        }
        let freed = format::size(plan.freed, self.text);
        let summary = (self.text.plan_summary)(plan.remove.len(), &freed);
        wrapped(ui, styled(summary, Type::Body, palette.text_primary));
        ScrollArea::vertical()
            .max_height(du(36.0))
            .id_salt("packages")
            .show(ui, |ui| {
                terminal(ui, |ui| {
                    wrapped(
                        ui,
                        RichText::new(plan.remove.join("  "))
                            .monospace()
                            .color(TERMINAL_FOREGROUND),
                    );
                });
            });
    }

    fn other_section(&self, ui: &mut Ui, palette: &Palette) {
        let text = match &self.item.removal {
            Removal::Pacman | Removal::Unavailable { .. } => return,
            Removal::Manager { name, .. } => (self.text.through_manager)(name),
            Removal::Flatpak { .. } => self.text.flatpak_note.to_owned(),
            Removal::Steam { .. } => self.text.steam_note.to_owned(),
            Removal::Cargo { .. } => self.text.cargo_note.to_owned(),
            Removal::Files { paths, access } => {
                let list = paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                let intro = match access {
                    Access::User => self.text.files_go_to_bin,
                    Access::Root => self.text.files_go_for_good,
                };
                format!("{intro}\n{list}")
            }
        };
        wrapped(ui, styled(text, Type::Body, palette.text_primary));
        if is_irreversible(&self.item) {
            warning(ui, self.text.not_a_package, palette.error);
        }
    }

    fn command_section(&self, ui: &mut Ui) {
        if let Phase::Ready { spec: Some(spec), .. } = &self.phase {
            terminal(ui, |ui| {
                wrapped(
                    ui,
                    RichText::new(&spec.shown).monospace().color(TERMINAL_FOREGROUND),
                );
            });
        }
    }

    fn log_section(&self, ui: &mut Ui) {
        if self.log.is_empty() {
            return;
        }
        ScrollArea::vertical()
            .max_height(du(40.0))
            .id_salt("log")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                terminal(ui, |ui| {
                    for line in &self.log {
                        wrapped(ui, RichText::new(line).monospace().color(TERMINAL_FOREGROUND));
                    }
                });
            });
    }

    fn status_line(&self, ui: &mut Ui, palette: &Palette) {
        match &self.phase {
            Phase::Checking(_) => wrapped(ui, styled(self.text.checking, Type::Body, palette.pending)),
            Phase::Running(_) => wrapped(ui, styled(self.text.removing, Type::Body, palette.pending)),
            Phase::Succeeded(_) => wrapped(
                ui,
                styled(
                    format!("{} {}", glyph::CHECK, self.text.done),
                    Type::Body,
                    palette.success,
                ),
            ),
            Phase::Failed(message) => {
                warning(ui, (self.text.failed)(message), palette.error);
            }
            Phase::Ready {
                refusal: Some(text), ..
            } => warning(ui, text.clone(), palette.error),
            Phase::Ready { .. } => {}
        }
        if let Some(note) = &self.note {
            wrapped(ui, styled(note.clone(), Type::Body, palette.pending));
        }
    }

    /// Buttons on the right edge. Cancelling is locked only while something is running.
    fn buttons(&mut self, ui: &mut Ui, palette: &Palette) -> Option<Exit> {
        let mut exit = None;
        let spec = match &self.phase {
            Phase::Ready { spec, .. } => spec.clone(),
            _ => None,
        };
        let confirmed = !self.needs_risk_confirmation() || self.confirmed_risk;
        let busy = matches!(self.phase, Phase::Running(_) | Phase::Succeeded(_));
        let failed = matches!(self.phase, Phase::Failed(_));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !failed {
                let ready = spec.is_some() && confirmed;
                let confirm = action_button(
                    ui,
                    self.text.remove,
                    Some(glyph::TRASH),
                    Tone::Danger,
                    ready,
                    palette,
                );
                if confirm.clicked()
                    && let Some(spec) = &spec
                {
                    self.note = None;
                    self.log.clear();
                    self.phase = Phase::Running(self.backend.run(spec));
                }
            }
            let close_label = if failed { self.text.close } else { self.text.cancel };
            if action_button(ui, close_label, None, Tone::Plain, !busy, palette).clicked() {
                exit = Some(Exit::Cancelled);
            }
            if let Some(spec) = &spec {
                copy_button(ui, self.text.copy_command, &spec.shown, palette);
            }
        });
        exit
    }

    fn body(&mut self, ui: &mut Ui, palette: &Palette) -> Option<Exit> {
        ui.spacing_mut().item_spacing.y = PAD_CARD;
        let title = (self.text.confirm_title)(&self.item.title);
        ui.add(
            egui::Label::new(weighted(
                title,
                Type::AppTitle,
                Weight::Bold,
                palette.text_primary,
            ))
            .wrap(),
        );
        if self.item.usage == Usage::Running {
            warning(ui, self.text.still_running, palette.pending);
        }
        if let Some(text) = self.item.warning.clone() {
            warning(ui, text, palette.pending);
        }
        self.pacman_section(ui, palette);
        self.other_section(ui, palette);
        self.command_section(ui);
        self.log_section(ui);
        self.status_line(ui, palette);
        if self.needs_risk_confirmation() && matches!(self.phase, Phase::Ready { spec: Some(_), .. }) {
            let text = styled(self.text.understood, Type::Body, palette.text_primary);
            ui.checkbox(&mut self.confirmed_risk, text);
        }
        ui.add_space(PAD_TIGHT);
        ui.allocate_ui(vec2(ui.available_width(), du(10.0)), |ui| {
            self.buttons(ui, palette)
        })
        .inner
    }

    /// Draws the dialog. `Some` means it is over.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<Exit> {
        self.poll();
        if let Phase::Succeeded(at) = self.phase {
            if at.elapsed() >= SUCCESS_HOLD {
                return Some(Exit::Removed(self.item.title.clone()));
            }
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        let width = clamp_width(ctx.content_rect().width(), 70.0, 0.88, 120.0);
        let mut exit = None;
        let modal = Modal::new(Id::unique("removal"))
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let frame = Frame::new()
                    .fill(palette.surface)
                    .inner_margin(Margin::same(MARGIN_SCREEN as i8))
                    .show(ui, |ui| {
                        ui.set_width(width - 2.0 * MARGIN_SCREEN);
                        exit = self.body(ui, palette);
                    });
                state_border(ui, frame.response.rect, self.state(), palette);
            });
        let running = matches!(self.phase, Phase::Running(_) | Phase::Succeeded(_));
        if modal.should_close() && !running && exit.is_none() {
            exit = Some(Exit::Cancelled);
        }
        if matches!(self.phase, Phase::Checking(_) | Phase::Running(_)) {
            ctx.request_repaint();
        }
        exit
    }
}
