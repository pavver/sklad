//! The window. `App` holds the state; the modules draw parts of it and report what was asked for.

mod catalogue;
mod detail;
mod dialog;
pub mod format;
mod header;
mod list;
mod operator;
mod pictures;
mod side_bar;
pub mod view;

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, CentralPanel, Frame, Id, Panel, Ui, vec2};

use crate::i18n::{Choice, Lang, Text};
use crate::launch;
use crate::model::{DockerKind, Item};
use crate::remove::{self, Backend, Event, command_line};
use crate::scan::Catalog;
use dialog::{Dialog, Exit};
use operator::feedback::Tone;
use operator::icon;
use operator::split_pane::{Pane, SplitPane};
use operator::state::{Activity, Outcome};
use operator::toast::Toasts;
use operator::tokens::{DARK, LIGHT, Palette, du, install_fonts};
use pictures::Pictures;
use view::View;

/// Produces a catalogue. Runs on a background thread.
pub type Scanner = Arc<dyn Fn(Lang) -> Catalog + Send + Sync>;

/// The card of the selected item, with the command worked out once on selection.
struct Selection {
    id: String,
    command: Option<String>,
}

/// How a reading of the system under way shows itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    /// The catalogue is read anew: the list and the side bar give way to skeletons.
    Anew,
    /// After a removal: the catalogue stays on screen, and the result updates it in place.
    Quietly,
}

struct Scanning {
    result: Receiver<Catalog>,
    reading: Reading,
}

/// A launch under way: whose it is and what its command reports.
struct Launching {
    title: String,
    events: Receiver<Event>,
    said: Option<String>,
}

pub struct App {
    backend: Arc<dyn Backend>,
    scanner: Scanner,
    scanning: Option<Scanning>,
    catalog: Option<Arc<Catalog>>,
    view: View,
    visible: Vec<usize>,
    selection: Option<Selection>,
    pictures: Pictures,
    dialog: Option<Dialog>,
    toasts: Toasts,
    refresh: Activity,
    launching: Option<Launching>,
    /// The launch the card shows, and the item it belongs to.
    launch: Activity,
    launched: Option<String>,
    dark: bool,
    applied: Option<bool>,
    /// The widths of the side bar and of the card, in points. Each stays as the person set it,
    /// and the list between them takes what the window gives or takes.
    side: f32,
    card: f32,
    choice: Choice,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs() as i64)
}

const SIDE_BAR: f32 = du(84.0);
/// The narrowest the side bar gets, in DU: its labels still fit.
const SIDE_BAR_MIN: f32 = 64.0;
/// The width of the card at first and after a double click on its divider.
const CARD: f32 = du(112.0);

/// What a row shows in its avatar: the icon of its kind for a Docker object, which has no
/// picture, and the picture of the program otherwise.
pub(crate) fn picture<'a>(item: &Item, pictures: &'a mut Pictures) -> operator::content::Picture<'a> {
    use operator::content::Picture;
    match item.docker {
        Some(DockerKind::Project) => Picture::Glyph(icon::ACCOUNT_TREE),
        Some(DockerKind::Container) => Picture::Glyph(icon::BOX),
        Some(DockerKind::Image) => Picture::Glyph(icon::STACKS),
        Some(DockerKind::Volume) => Picture::Glyph(icon::DATABASE),
        None => pictures.get(item.icon.as_deref()),
    }
}

impl App {
    pub fn new(ctx: &egui::Context, backend: Arc<dyn Backend>, scanner: Scanner, choice: Choice) -> Self {
        let dark = !matches!(ctx.system_theme(), Some(egui::Theme::Light));
        let mut app = Self {
            backend,
            scanner,
            scanning: None,
            catalog: None,
            view: View::default(),
            visible: Vec::new(),
            selection: None,
            pictures: Pictures::new(ctx),
            dialog: None,
            toasts: Toasts::default(),
            refresh: Activity::default(),
            launching: None,
            launch: Activity::default(),
            launched: None,
            dark,
            applied: None,
            side: SIDE_BAR,
            card: CARD,
            choice,
        };
        app.rescan(ctx, Reading::Anew);
        app
    }

    fn text(&self) -> &'static Text {
        self.choice.lang.text()
    }

    fn palette(&self) -> &'static Palette {
        if self.dark { &DARK } else { &LIGHT }
    }

    /// Descriptions of programs come in the language of the scan, so the catalogue is read again.
    fn switch_language(&mut self, ctx: &egui::Context, lang: Lang) {
        self.choice.set(lang);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.text().app_name.to_owned()));
        self.rescan(ctx, Reading::Anew);
    }

    /// Reads the system again on a background thread. A reading started later supersedes this one.
    fn rescan(&mut self, ctx: &egui::Context, reading: Reading) {
        let (sender, receiver) = channel();
        let scanner = Arc::clone(&self.scanner);
        let lang = self.choice.lang;
        let repaint = ctx.clone();
        thread::spawn(move || {
            let _ = sender.send(scanner(lang));
            repaint.request_repaint();
        });
        self.scanning = Some(Scanning {
            result: receiver,
            reading,
        });
        if reading == Reading::Anew {
            self.refresh.start(ctx.input(|input| input.time));
        }
    }

    fn refilter(&mut self) {
        self.visible = self
            .catalog
            .as_ref()
            .map(|catalog| view::visible(&catalog.items, &self.view))
            .unwrap_or_default();
    }

    fn receive_scan(&mut self, now: f64) {
        let Some(scanning) = &self.scanning else {
            return;
        };
        if let Ok(catalog) = scanning.result.try_recv() {
            let reading = scanning.reading;
            self.scanning = None;
            self.catalog = Some(Arc::new(match (&self.catalog, reading) {
                (Some(old), Reading::Quietly) => old.updated(catalog),
                _ => catalog,
            }));
            if reading == Reading::Anew {
                self.refresh.finish(Outcome::Success, now);
            }
            self.refilter();
            let selected = self.selection.as_ref().map(|selection| selection.id.clone());
            self.selection = None;
            if let Some(id) = selected {
                self.select(&id);
            }
        }
    }

    fn start_launch(&mut self, item: &Item, now: f64) {
        let Some(how) = &item.launch else {
            return;
        };
        if self.launching.is_some() {
            return;
        }
        self.launching = Some(Launching {
            title: item.title.clone(),
            events: self.backend.run(&launch::spec(how)),
            said: None,
        });
        self.launch = Activity::default();
        self.launch.start(now);
        self.launched = Some(item.id.clone());
    }

    /// Takes what the launch command reported. It keeps the button pending, so it repaints.
    fn receive_launch(&mut self, ctx: &egui::Context, now: f64) {
        let text = self.text();
        let Some(launching) = &mut self.launching else {
            return;
        };
        let mut finished = None;
        while let Ok(event) = launching.events.try_recv() {
            match event {
                Event::Line(line) => launching.said = Some(line),
                Event::Finished(outcome) => finished = Some(outcome),
            }
        }
        let Some(outcome) = finished else {
            ctx.request_repaint_after(Duration::from_millis(50));
            return;
        };
        if outcome == remove::Outcome::Done {
            self.launch.finish(Outcome::Success, now);
        } else {
            self.launch.finish(Outcome::Error, now);
            let mut message = (text.launch_failed)(&launching.title);
            if let Some(said) = &launching.said {
                message = format!("{message}: {said}");
            }
            self.toasts.message(Tone::Error, message);
        }
        self.launching = None;
    }

    fn selected_item(&self) -> Option<&Item> {
        let id = &self.selection.as_ref()?.id;
        self.catalog.as_ref()?.items.iter().find(|item| &item.id == id)
    }

    fn select(&mut self, id: &str) {
        let Some(catalog) = &self.catalog else {
            return;
        };
        self.selection = catalog
            .items
            .iter()
            .find(|item| item.id == id)
            .map(|item| Selection {
                id: item.id.clone(),
                command: command_line(item, &catalog.db),
            });
    }

    /// Whether the catalogue is being read anew, the first time or again: the list and the side
    /// bar show skeletons then, and the filters are hidden.
    fn loading(&self) -> bool {
        self.catalog.is_none()
            || self
                .scanning
                .as_ref()
                .is_some_and(|scanning| scanning.reading == Reading::Anew)
    }

    /// Takes out of the catalogue what a removal took, and reads the system quietly for the rest.
    fn removed(&mut self, ctx: &egui::Context, gone: &[String]) {
        if let Some(catalog) = &self.catalog {
            self.catalog = Some(Arc::new(catalog.without(gone)));
            self.refilter();
        }
        self.rescan(ctx, Reading::Quietly);
    }

    /// What a press on the card asks for.
    fn act(&mut self, ctx: &egui::Context, item: Item, action: detail::Action, time: f64) {
        match action {
            detail::Action::Launch => self.start_launch(&item, time),
            detail::Action::Remove => self.open_dialog(item),
            detail::Action::Close => self.selection = None,
            detail::Action::Open(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            detail::Action::Follow(name) => self.follow(&item, &name),
        }
    }

    fn open_dialog(&mut self, item: Item) {
        if let Some(catalog) = &self.catalog {
            self.dialog = Some(Dialog::open(
                item,
                Arc::clone(catalog),
                Arc::clone(&self.backend),
                self.choice.lang.text(),
            ));
        }
    }

    /// Selects a neighbour, which comes from the same source: a package names packages, a
    /// Docker row names Docker rows.
    fn follow(&mut self, item: &Item, name: &str) {
        let id = self.catalog.as_ref().and_then(|catalog| {
            catalog
                .items
                .iter()
                .find(|other| other.source == item.source && other.package == name)
                .map(|other| other.id.clone())
        });
        self.select(&id.unwrap_or_else(|| format!("pacman:{name}")));
    }

    /// One frame of the whole window.
    pub fn draw(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let palette = self.palette();
        if self.applied.is_none() {
            install_fonts(&ctx);
        }
        if self.applied != Some(self.dark) {
            operator::apply(&ctx, palette);
            self.applied = Some(self.dark);
        }
        let time = ctx.input(|input| input.time);
        self.receive_scan(time);
        self.receive_launch(&ctx, time);
        self.pictures.receive(&ctx);
        let now = unix_now();

        Panel::top("header")
            .frame(Frame::NONE)
            .show(ui, |ui| self.header(ui, palette));

        let centre = Frame::new().fill(palette.background);
        CentralPanel::default().frame(centre).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let width = ui.available_width().max(1.0);
            let mut share = self.side / width;
            let sections = SplitPane {
                id: Id::unique("sections and catalogue"),
                initial: SIDE_BAR / width,
                min_size: SIDE_BAR_MIN,
            };
            let moved = sections.show(ui, &mut share, palette, |ui, pane| match pane {
                Pane::Start => self.side_bar(ui, palette),
                Pane::End => self.catalogue(ui, now, palette),
            });
            if moved {
                self.side = share * width;
            }
        });

        let exit = self.dialog.as_mut().and_then(|dialog| dialog.show(&ctx, palette));
        match exit {
            Some(Exit::Removed { title, gone }) => {
                self.dialog = None;
                self.selection = None;
                self.toasts.message(Tone::Success, (self.text().removed)(&title));
                self.removed(&ctx, &gone);
            }
            Some(Exit::Cancelled) => self.dialog = None,
            None => {}
        }
        self.toasts.show(&ctx, self.text().close, palette);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}
