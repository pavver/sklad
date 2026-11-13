//! The window. `App` holds the state; the modules draw parts of it and report what was asked for.

mod detail;
mod dialog;
pub mod format;
mod list;
mod operator;
mod pictures;
pub mod view;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, CentralPanel, Frame, Id, Panel, ScrollArea, Ui, vec2};

use crate::i18n::{Choice, Lang, Text};
use crate::launch;
use crate::model::{DockerKind, Item, Section, SourceKind};
use crate::remove::{self, Backend, Event, command_line};
use crate::scan::Catalog;
use dialog::{Dialog, Exit};
use operator::button::IconButton;
use operator::checkbox_group::{CheckboxGroup, GroupItem, IconSize, TileIcon};
use operator::collapsible::Collapsible;
use operator::combobox::{self, Combobox};
use operator::content::Picture;
use operator::content::{ChipColor, Pane, SplitPane, chip, input_chip, theme_toggle};
use operator::feedback::{Toasts, Tone};
use operator::icon::{self, Icon};
use operator::layout::{header, toolbar};
use operator::search_field::SearchField;
use operator::state::{Activity, Look, Outcome, result_hold};
use operator::tokens::{DARK, LIGHT, Palette, dim, du, install_fonts};
use pictures::Pictures;
use view::{Facet, Origin, View};

/// Produces a catalogue. Runs on a background thread.
pub type Scanner = Arc<dyn Fn(Lang) -> Catalog + Send + Sync>;

/// The card of the selected item, with the command worked out once on selection.
struct Selection {
    id: String,
    command: Option<String>,
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
    scanning: Option<Receiver<Catalog>>,
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
/// The narrowest the list and the card get, in DU.
const PANE_MIN: f32 = 90.0;

/// The largest number a tile of the side bar shows in full; a catalogue rarely holds more.
const COUNT_MAX: usize = 99_999;

fn origin_icon(origin: Origin) -> Icon {
    match origin {
        Origin::Explicit => icon::PERSON,
        Origin::Needed => icon::LINK,
        Origin::Orphan => icon::LINK_OFF,
    }
}

/// The names of the icon of a filter in the icon themes. Origins have none there.
fn theme_names(facet: Facet) -> &'static [&'static str] {
    match facet {
        Facet::Section(section) => section.theme_icons(),
        Facet::Source(source) => source.theme_icons(),
        Facet::Origin(_) => &[],
    }
}

/// The icon and the name of a filter, as its tile and its chip say it.
fn facet_label(facet: Facet, text: &Text) -> (Icon, &'static str) {
    match facet {
        Facet::Section(section) => (section_icon(section), (text.section)(section)),
        Facet::Origin(origin) => {
            let name = match origin {
                Origin::Explicit => text.origin_explicit,
                Origin::Needed => text.origin_needed,
                Origin::Orphan => text.origin_orphans,
            };
            (origin_icon(origin), name)
        }
        Facet::Source(source) => (source_icon(source), (text.source)(source)),
    }
}

fn section_icon(section: Section) -> Icon {
    match section {
        Section::Internet => icon::PUBLIC,
        Section::Multimedia => icon::MOVIE,
        Section::Graphics => icon::PALETTE,
        Section::Office => icon::DESCRIPTION,
        Section::Development => icon::CODE,
        Section::Games => icon::SPORTS_ESPORTS,
        Section::Education => icon::SCHOOL,
        Section::Science => icon::SCIENCE,
        Section::System => icon::SETTINGS,
        Section::Utilities => icon::BUILD,
        Section::Console => icon::TERMINAL,
        Section::Libraries => icon::EXTENSION,
        Section::Fonts => icon::FONT_DOWNLOAD,
        Section::Drivers => icon::MEMORY,
        Section::Meta => icon::LAYERS,
    }
}

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

fn source_icon(source: SourceKind) -> Icon {
    match source {
        SourceKind::Repo => icon::INVENTORY_2,
        SourceKind::Aur => icon::GROUPS,
        SourceKind::Flatpak => icon::DEPLOYED_CODE,
        SourceKind::Steam => icon::SPORTS_ESPORTS,
        SourceKind::Cargo | SourceKind::Node | SourceKind::Python => icon::CODE,
        SourceKind::Rustup => icon::CONSTRUCTION,
        SourceKind::LocalDeb => icon::PACKAGE_2,
        SourceKind::Docker => icon::DIRECTIONS_BOAT,
        SourceKind::Manual => icon::FOLDER,
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
        app.rescan(ctx);
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
        self.rescan(ctx);
    }

    fn rescan(&mut self, ctx: &egui::Context) {
        let (sender, receiver) = channel();
        let scanner = Arc::clone(&self.scanner);
        let lang = self.choice.lang;
        let repaint = ctx.clone();
        thread::spawn(move || {
            let _ = sender.send(scanner(lang));
            repaint.request_repaint();
        });
        self.scanning = Some(receiver);
        self.refresh.start(ctx.input(|input| input.time));
    }

    fn refilter(&mut self) {
        self.visible = self
            .catalog
            .as_ref()
            .map(|catalog| view::visible(&catalog.items, &self.view))
            .unwrap_or_default();
    }

    fn receive_scan(&mut self, now: f64) {
        let finished = self
            .scanning
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(catalog) = finished {
            self.catalog = Some(Arc::new(catalog));
            self.scanning = None;
            self.refresh.finish(Outcome::Success, now);
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

    fn header(&mut self, ui: &mut Ui, palette: &Palette) {
        let text = self.text();
        // What the list shows, and how much it takes: all of it, or the part the filters and the
        // search leave.
        let subtitle = match &self.catalog {
            Some(catalog) => {
                let shown = &self.visible;
                let size: u64 = shown.iter().filter_map(|&index| catalog.items[index].size).sum();
                let count = if shown.len() == catalog.items.len() {
                    (text.records)(shown.len())
                } else {
                    (text.shown_of)(shown.len(), catalog.items.len())
                };
                format!("{count}, {}", format::size(size, text))
            }
            None => text.reading_system.to_owned(),
        };
        let mut toggled = false;
        let mut language = None;
        let mut refresh = false;
        let mut query = None;
        let look = self.refresh.look(ui.input(|input| input.time), result_hold());
        header(ui, text.app_name, Some(&subtitle), palette, |ui| {
            toggled = theme_toggle(ui, Id::unique("theme"), self.dark, text.dark_theme, palette);
            let names: Vec<String> = Lang::ALL.iter().map(|lang| lang.name().to_owned()).collect();
            let selected = Lang::ALL
                .iter()
                .position(|&lang| lang == self.choice.lang)
                .unwrap_or(0);
            let choice = Combobox {
                id: Id::unique("language"),
                options: &names,
                label: text.language,
                no_matches: text.no_matches,
                width: combobox::natural_width(ui, &names),
            }
            .show(ui, selected, palette);
            language = choice.and_then(|index| Lang::ALL.get(index).copied());
            let field = SearchField {
                id: Id::unique("search"),
                placeholder: text.search,
                clear_label: text.clear_search,
                height: dim::ROW_COMPACT,
                width: (ui.available_width() - du(dim::ROW_COMPACT + dim::GUTTER)).clamp(0.0, du(80.0)),
            };
            query = field.show(ui, &mut self.view.query, palette);
            refresh = IconButton::new(icon::REFRESH, text.refresh)
                .look(look)
                .show(ui, palette)
                .clicked();
        });
        if refresh && self.scanning.is_none() {
            self.rescan(ui.ctx());
        }
        if query.is_some() {
            self.refilter();
        }
        if toggled {
            self.dark = !self.dark;
        }
        if let Some(lang) = language {
            self.switch_language(ui.ctx(), lang);
        }
    }

    /// One group of filters: a `Collapsible` holding a `CheckboxGroup`, each tile with the number
    /// choosing it would show. The header counts the filters of the group that are on, so a
    /// closed group still says it filters. `key` names the group whatever the language.
    fn facets(&mut self, ui: &mut Ui, key: &str, title: &str, facets: &[Facet], palette: &Palette) {
        if facets.is_empty() {
            return;
        }
        let text = self.text();
        let items = self
            .catalog
            .as_ref()
            .map_or(&[][..], |catalog| catalog.items.as_slice());
        let theme = self.catalog.as_ref().map(|catalog| &catalog.theme_icons);
        let pictures = &mut self.pictures;
        let tiles: Vec<GroupItem<'_>> = facets
            .iter()
            .map(|&facet| {
                let (glyph, name) = facet_label(facet, text);
                // The icon of the system theme, so the side bar looks like the rest of the
                // desktop; the glyph while it loads, or when the theme has none.
                let path = theme.and_then(|found| theme_names(facet).iter().find_map(|name| found.get(name)));
                let icon = match pictures.get(path.map(PathBuf::as_path)) {
                    Picture::Ready(texture) => TileIcon::Picture(texture.clone()),
                    _ => TileIcon::Glyph(glyph),
                };
                GroupItem {
                    name,
                    icon: Some(icon),
                    count: Some(self.view.count(items, facet)),
                    selected: self.view.is_on(facet),
                }
            })
            .collect();
        let section = Collapsible {
            id: Id::unique(("filter group", key)),
            title,
            count: tiles.iter().filter(|tile| tile.selected).count(),
            counter_max: COUNT_MAX,
            initially_open: true,
        };
        let mut clicked = None;
        section.show(ui, palette, |ui| {
            let group = CheckboxGroup {
                id: Id::unique(("facets", key)),
                items: &tiles,
                icon_size: IconSize::Large,
                counter_max: COUNT_MAX,
            };
            clicked = group.show(ui, palette);
        });
        if let Some(index) = clicked {
            self.view.toggle(facets[index]);
            self.refilter();
        }
    }

    fn side_bar(&mut self, ui: &mut Ui, palette: &Palette) {
        let text = self.text();
        let (programs, other, sources): (Vec<Facet>, Vec<Facet>, Vec<Facet>) =
            self.catalog.as_ref().map_or_else(Default::default, |catalog| {
                let items = &catalog.items;
                let sections = |list: &[Section]| -> Vec<Facet> {
                    view::offered_sections(items, list, &self.view)
                        .into_iter()
                        .map(Facet::Section)
                        .collect()
                };
                let sources: Vec<Facet> = view::offered_sources(items, catalog.system, &self.view)
                    .into_iter()
                    .map(Facet::Source)
                    .collect();
                (sections(&Section::APPS), sections(&Section::OTHER), sources)
            });
        let origins = Origin::ALL.map(Facet::Origin);
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.facets(ui, "programs", text.heading_programs, &programs, palette);
            self.facets(ui, "other", text.other, &other, palette);
            self.facets(ui, "origin", text.heading_origin, &origins, palette);
            self.facets(ui, "source", text.source_heading, &sources, palette);
        });
    }

    /// The chosen filters as chips, each with its remove button, and a chip at the end that
    /// takes them all off. Nothing when no filter is on.
    fn filter_bar(&mut self, ui: &mut Ui, palette: &Palette) {
        if self.view.filters().is_empty() {
            return;
        }
        let text = self.text();
        let mut removed = None;
        let mut reset = false;
        let filters = self.view.filters().to_vec();
        toolbar(
            ui,
            text.filters,
            palette,
            |ui| {
                ScrollArea::horizontal()
                    .id_salt("filter chips")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = du(dim::PAD_TIGHT);
                            for facet in filters {
                                let (_, name) = facet_label(facet, text);
                                let id = Id::unique(("filter chip", name));
                                if input_chip(ui, id, name, &(text.remove_filter)(name), palette) {
                                    removed = Some(facet);
                                }
                            }
                        });
                    });
            },
            |ui| reset = chip(ui, text.reset_filters, ChipColor::Neutral, true, palette).clicked(),
        );
        if let Some(facet) = removed {
            self.view.toggle(facet);
            self.refilter();
        }
        if reset {
            self.view.clear();
            self.refilter();
        }
    }

    fn list(&mut self, ui: &mut Ui, now: i64, palette: &Palette) {
        let empty = Vec::new();
        let catalog = self.catalog.clone();
        let items = catalog.as_ref().map_or(&empty, |catalog| &catalog.items);
        let rows = list::Rows {
            items,
            indices: &self.visible,
            selected: self.selection.as_ref().map(|selection| selection.id.as_str()),
            sorting: list::sorting_of(&self.view),
            loading: self.scanning.is_some(),
            now,
            text: self.text(),
        };
        match list::show(ui, &rows, &mut self.pictures, palette) {
            // The selected row pressed again closes the card.
            Some(list::Event::Select(id)) => {
                if self
                    .selection
                    .as_ref()
                    .is_some_and(|selection| selection.id == id)
                {
                    self.selection = None;
                } else {
                    self.select(&id);
                }
            }
            Some(list::Event::Sort(sorting)) => {
                list::sort_by(&mut self.view, sorting);
                self.refilter();
            }
            None => {}
        }
    }

    fn card(&mut self, ui: &mut Ui, now: i64, palette: &Palette) {
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        let time = ui.input(|input| input.time);
        let launch = if self.launched.as_deref() == Some(item.id.as_str()) {
            self.launch.look(time, result_hold())
        } else {
            Look::IDLE
        };
        let card = detail::Card {
            item: &item,
            launch,
            command: self
                .selection
                .as_ref()
                .and_then(|selection| selection.command.as_deref()),
            now,
            text: self.text(),
        };
        match detail::show(ui, &card, &mut self.pictures, palette) {
            Some(detail::Action::Launch) => self.start_launch(&item, time),
            Some(detail::Action::Remove) => {
                if let Some(catalog) = &self.catalog {
                    self.dialog = Some(Dialog::open(
                        item,
                        Arc::clone(catalog),
                        Arc::clone(&self.backend),
                        self.choice.lang.text(),
                    ));
                }
            }
            Some(detail::Action::Close) => self.selection = None,
            Some(detail::Action::Open(url)) => ui.ctx().open_url(egui::OpenUrl::new_tab(url)),
            Some(detail::Action::Follow(name)) => {
                // A neighbour comes from the same source: a package names packages, a Docker
                // row names Docker rows.
                let id = self.catalog.as_ref().and_then(|catalog| {
                    catalog
                        .items
                        .iter()
                        .find(|other| other.source == item.source && other.package == name)
                        .map(|other| other.id.clone())
                });
                self.select(&id.unwrap_or_else(|| format!("pacman:{name}")));
            }
            None => {}
        }
    }

    /// The tool bar over the list, and the card beside it when something is selected.
    fn catalogue(&mut self, ui: &mut Ui, now: i64, palette: &Palette) {
        self.filter_bar(ui, palette);
        if self.selection.is_some() {
            let width = ui.available_width().max(1.0);
            let mut share = 1.0 - self.card / width;
            let pane = SplitPane {
                id: Id::unique("list and card"),
                initial: 1.0 - CARD / width,
                min_size: PANE_MIN,
            };
            let moved = pane.show(ui, &mut share, palette, |ui, side| match side {
                Pane::Start => self.list(ui, now, palette),
                Pane::End => self.card(ui, now, palette),
            });
            if moved {
                self.card = (1.0 - share) * width;
            }
        } else {
            self.list(ui, now, palette);
        }
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
            Some(Exit::Removed(title)) => {
                self.dialog = None;
                self.selection = None;
                self.toasts.message(Tone::Success, (self.text().removed)(&title));
                self.rescan(&ctx);
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
