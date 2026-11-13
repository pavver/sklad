//! The removal dialog: shows exactly what will happen, then does it and shows the output.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;

use eframe::egui::{self, Id, Sense, Ui, vec2};

use super::format;
use super::operator::code::{CodeBlock, console_terminal};
use super::operator::content::checkbox;
use super::operator::dialog::{Asked, Dialog as Shell, dialog_hold};
use super::operator::feedback::{Tone, banner, paint_indeterminate};
use super::operator::icon;
use super::operator::segmented::{Segment, natural_width, segmented};
use super::operator::state::{Activity, Outcome};
use super::operator::tokens::{Palette, Type, dim, du, text};
use crate::i18n::Text;
use crate::model::{Access, Item, Removal, Usage};
use crate::plan::{Mode, Plan};
use crate::remove::{Backend, Event, Outcome as RunOutcome, Spec, is_irreversible, other_spec, package_spec};
use crate::scan::Catalog;

type Preview = Result<Vec<String>, String>;

enum Phase {
    /// Pacman is asked whether it accepts the list.
    Checking(Receiver<Preview>),
    /// The person decides. `spec` is empty when there is nothing that can be run.
    Ready {
        spec: Option<Spec>,
        refusal: Option<String>,
    },
    Running(Receiver<Event>, Spec),
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
    plan: Option<Plan>,
    phase: Phase,
    confirmed_risk: bool,
    /// Why the last attempt did not remove anything.
    failure: Option<String>,
    note: Option<String>,
    log: Vec<String>,
    activity: Activity,
    succeeded: Option<f64>,
    opened: Option<f64>,
}

fn wrapped(ui: &mut Ui, content: impl Into<String>, palette: &Palette) {
    ui.add(egui::Label::new(text(content, Type::Body, palette.text_primary)).wrap());
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
            failure: None,
            note: None,
            log: Vec::new(),
            activity: Activity::default(),
            succeeded: None,
            opened: None,
        };
        dialog.prepare();
        dialog
    }

    fn prepare(&mut self) {
        self.confirmed_risk = false;
        let Removal::Package(system) = self.item.removal else {
            let spec = other_spec(&self.item.removal);
            let refusal = spec.is_none().then(|| self.text.path_too_close.to_owned());
            self.phase = Phase::Ready { spec, refusal };
            return;
        };
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
            let _ = sender.send(backend.preview(system, &names));
        });
        self.phase = Phase::Checking(receiver);
    }

    /// The package manager must return the same list: any difference means the plan is not what
    /// would happen.
    fn review(&self, answer: Preview) -> Phase {
        let (Some(plan), Removal::Package(system)) = (&self.plan, &self.item.removal) else {
            return Phase::Ready {
                spec: None,
                refusal: None,
            };
        };
        match answer {
            Ok(list) if list == plan.remove => Phase::Ready {
                spec: Some(package_spec(plan, *system)),
                refusal: None,
            },
            Ok(_) => Phase::Ready {
                spec: None,
                refusal: Some((self.text.other_list_refused)(system.manager())),
            },
            Err(message) => Phase::Ready {
                spec: None,
                refusal: Some((self.text.manager_refused)(system.manager(), &message)),
            },
        }
    }

    fn poll(&mut self, now: f64) {
        match &self.phase {
            Phase::Checking(receiver) => {
                if let Ok(answer) = receiver.try_recv() {
                    self.phase = self.review(answer);
                }
            }
            Phase::Running(receiver, spec) => {
                let spec = spec.clone();
                let events: Vec<Event> = receiver.try_iter().collect();
                for event in events {
                    match event {
                        Event::Line(line) => self.log.push(line),
                        Event::Finished(RunOutcome::Done) => {
                            self.activity.finish(Outcome::Success, now);
                            self.succeeded = Some(now);
                        }
                        Event::Finished(RunOutcome::Failed(failure)) => {
                            self.activity.finish(Outcome::Error, now);
                            self.failure = Some((self.text.failed)(&self.text.failure(&failure)));
                            self.phase = Phase::Ready {
                                spec: Some(spec.clone()),
                                refusal: None,
                            };
                        }
                        Event::Finished(RunOutcome::Dismissed) => {
                            self.activity.finish(Outcome::Error, now);
                            self.note = Some(self.text.no_password.to_owned());
                            self.prepare();
                        }
                    }
                }
            }
            Phase::Ready { .. } => {}
        }
    }

    fn needs_risk_confirmation(&self) -> bool {
        is_irreversible(&self.item) || self.plan.as_ref().is_some_and(|plan| !plan.critical.is_empty())
    }

    fn pacman_section(&mut self, ui: &mut Ui, palette: &Palette) {
        let Some(plan) = self.plan.clone() else {
            return;
        };
        if plan.is_blocked() {
            let names = plan.blockers.join(", ");
            banner(
                ui,
                Tone::Warning,
                None,
                &(self.text.needed_by_programs)(&names),
                palette,
            );
        }
        if plan.is_blocked() || self.mode == Mode::WithDependents {
            let selected = usize::from(self.mode == Mode::WithDependents);
            let items = [
                Segment::new(self.text.keep_everything),
                Segment::new(self.text.remove_with_them),
            ];
            let width = natural_width(ui, &items).min(ui.available_width());
            let choice = segmented(
                ui,
                Id::unique("removal mode"),
                &items,
                selected,
                width,
                9.0,
                palette,
            );
            if let Some(choice) = choice {
                self.mode = if choice == 1 {
                    Mode::WithDependents
                } else {
                    Mode::Keep
                };
                self.failure = None;
                self.prepare();
                return;
            }
        }
        if plan.is_blocked() {
            return;
        }
        if !plan.metas.is_empty() {
            let metas = plan.metas.join(", ");
            let kept = plan.keep.join(", ");
            wrapped(ui, (self.text.held_by_meta)(&metas, &kept), palette);
        }
        if !plan.critical.is_empty() {
            let names = plan.critical.join(", ");
            banner(
                ui,
                Tone::Error,
                None,
                &(self.text.critical_packages)(&names),
                palette,
            );
        }
        let freed = format::size(plan.freed, self.text);
        wrapped(ui, (self.text.plan_summary)(plan.remove.len(), &freed), palette);
        CodeBlock {
            id: Id::unique("packages"),
            code: &plan.remove.join("  "),
            language: None,
            wrap: true,
            copy: None,
            max_height: Some(36.0),
        }
        .show(ui, palette);
    }

    fn other_section(&self, ui: &mut Ui, palette: &Palette) {
        let Some(content) = other_note(&self.item.removal, self.text) else {
            return;
        };
        wrapped(ui, content, palette);
        if is_irreversible(&self.item) {
            let loss = match self.item.removal {
                Removal::Docker { .. } => self.text.docker_data_lost,
                _ => self.text.not_a_package,
            };
            banner(ui, Tone::Error, None, loss, palette);
        }
    }

    fn spec(&self) -> Option<&Spec> {
        match &self.phase {
            Phase::Ready { spec, .. } => spec.as_ref(),
            Phase::Running(_, spec) => Some(spec),
            Phase::Checking(_) => None,
        }
    }

    fn status(&self, ui: &mut Ui, palette: &Palette) {
        match &self.phase {
            Phase::Checking(_) => {
                let (bar, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), du(dim::PAD_TIGHT)), Sense::hover());
                paint_indeterminate(ui, bar, palette);
                let manager = match self.item.removal {
                    Removal::Package(system) => system.manager(),
                    _ => "",
                };
                ui.label(text(
                    (self.text.checking)(manager),
                    Type::Caption,
                    palette.text_secondary,
                ));
            }
            Phase::Ready {
                refusal: Some(reason),
                ..
            } => banner(ui, Tone::Error, None, reason, palette),
            _ => {}
        }
        if let Some(failure) = &self.failure {
            banner(ui, Tone::Error, None, failure, palette);
        }
        if let Some(note) = &self.note {
            banner(ui, Tone::Warning, None, note, palette);
        }
    }

    fn body(&mut self, ui: &mut Ui, palette: &Palette) {
        if self.item.usage == Usage::Running {
            banner(ui, Tone::Warning, None, self.text.still_running, palette);
        }
        if let Some(warning) = self.item.warning.clone() {
            banner(ui, Tone::Warning, None, &warning, palette);
        }
        self.pacman_section(ui, palette);
        self.other_section(ui, palette);
        if let Some(spec) = self.spec() {
            CodeBlock {
                id: Id::unique("removal command"),
                code: &spec.shown,
                language: Some("sh"),
                wrap: true,
                copy: Some(self.text.copy_command),
                max_height: None,
            }
            .show(ui, palette);
        }
        if !self.log.is_empty() {
            console_terminal(ui, Id::unique("removal log"), &self.log, 40.0, palette);
        }
        self.status(ui, palette);
        let ready = matches!(self.phase, Phase::Ready { spec: Some(_), .. });
        if self.needs_risk_confirmation() && ready {
            checkbox(ui, &mut self.confirmed_risk, self.text.understood, palette);
        }
    }

    /// Draws the dialog. `Some` means it is over.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<Exit> {
        let now = ctx.input(|input| input.time);
        let opened = *self.opened.get_or_insert(now);
        self.poll(now);
        let look = self.activity.look(now, dialog_hold());
        if self
            .succeeded
            .is_some_and(|at| now - at >= f64::from(dialog_hold()))
        {
            return Some(Exit::Removed(self.item.title.clone()));
        }
        let confirmed = !self.needs_risk_confirmation() || self.confirmed_risk;
        let ready = matches!(self.phase, Phase::Ready { spec: Some(_), .. });
        let title = (self.text.confirm_title)(&self.item.title);
        let shell = Shell {
            id: Id::unique("removal"),
            title: &title,
            confirm_text: self.text.remove,
            cancel_text: self.text.cancel,
            close_label: self.text.close,
            has_confirm: true,
            confirm_icon: Some(icon::DELETE),
            confirm_enabled: ready && confirmed,
            destructive: true,
            look,
            opened,
        };
        let asked = shell.show(ctx, palette, |ui| self.body(ui, palette), |_| {});
        if matches!(self.phase, Phase::Checking(_)) || look.moves() {
            ctx.request_repaint();
        }
        match asked {
            Some(Asked::Close) => Some(Exit::Cancelled),
            Some(Asked::Confirm) => {
                if let Some(spec) = self.spec().cloned()
                    && ready
                    && confirmed
                {
                    self.note = None;
                    self.failure = None;
                    self.log.clear();
                    self.activity.start(now);
                    self.phase = Phase::Running(self.backend.run(&spec), spec);
                }
                None
            }
            None => None,
        }
    }
}

/// What the dialog says about a removal that is not one of the system packages: who removes
/// it, and what goes. `None` for system packages and for what cannot be removed.
fn other_note(removal: &Removal, text: &Text) -> Option<String> {
    Some(match removal {
        Removal::Package(_) | Removal::Unavailable { .. } => return None,
        Removal::Manager { name, .. } => (text.through_manager)(name),
        Removal::Flatpak { .. } => text.flatpak_note.to_owned(),
        Removal::Steam { .. } => text.steam_note.to_owned(),
        Removal::Cargo { .. } => text.cargo_note.to_owned(),
        Removal::Docker { parts } => {
            let list = lines(
                parts
                    .iter()
                    .map(|part| format!("{} {}", super::detail::docker_part(part.object, text), part.name)),
            );
            format!("{}\n{list}", (text.through_manager)("Docker"))
        }
        Removal::Files { paths, access } => {
            let intro = match access {
                Access::User => text.files_go_to_bin,
                Access::Root => text.files_go_for_good,
            };
            format!(
                "{intro}\n{}",
                lines(paths.iter().map(|path| path.display().to_string()))
            )
        }
    })
}

fn lines(items: impl Iterator<Item = String>) -> String {
    items.collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::model::{DockerObject, DockerPart};
    use std::path::PathBuf;

    #[test]
    fn the_dialog_names_who_removes_and_what_goes() {
        let text = Lang::En.text();
        assert_eq!(other_note(&Removal::Unavailable { reason: String::new() }, text), None);
        let files = Removal::Files {
            paths: vec![PathBuf::from("/home/a/x"), PathBuf::from("/home/a/y")],
            access: Access::User,
        };
        assert_eq!(
            other_note(&files, text).as_deref(),
            Some("The files will be moved to the bin:\n/home/a/x\n/home/a/y")
        );
        let part = |object, name: &str| DockerPart {
            object,
            id: name.to_owned(),
            name: name.to_owned(),
            size: None,
        };
        let docker = Removal::Docker {
            parts: vec![part(DockerObject::Container, "web"), part(DockerObject::Volume, "data")],
        };
        let note = other_note(&docker, text).unwrap_or_default();
        assert!(note.starts_with("Will be removed through Docker.\n"), "{note}");
        assert_eq!(note.lines().count(), 3, "one line per part: {note}");
        assert!(note.ends_with(" data"), "{note}");
    }
}
