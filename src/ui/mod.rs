//! The window. `App` holds the state; the modules draw parts of it and report what was asked for.

mod detail;
mod dialog;
pub mod format;
mod glyph;
mod list;
pub mod theme;
pub mod view;
mod widgets;

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, CentralPanel, Frame, Margin, Panel, ScrollArea, Ui, vec2};

use crate::i18n::{Choice, Lang, Text};
use crate::model::{Item, Section, SourceKind};
use crate::remove::{Backend, command_line};
use crate::scan::Catalog;
use dialog::{Dialog, Exit};
use theme::{
    DARK, GUTTER, LIGHT, MARGIN_SCREEN, PAD_CARD, PAD_SMALL, Palette, STANDARD_ROW, Type, Weight,
    clamp_width, du, styled, weighted,
};
use view::{Order, OriginFilter, Scope, SortKey, View};
use widgets::{
    IconCache, SEGMENT_HEIGHT, Tone, action_button, icon_button, nav_heading, nav_row, search_field,
    segmented,
};

/// Produces a catalogue. Runs on a background thread.
pub type Scanner = Arc<dyn Fn(Lang) -> Catalog + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Theme {
    Dark,
    Light,
}

impl Theme {
    fn palette(self) -> &'static Palette {
        match self {
            Theme::Dark => &DARK,
            Theme::Light => &LIGHT,
        }
    }
}

/// The card of the selected item, with the command worked out once on selection.
struct Selection {
    id: String,
    command: Option<String>,
}

pub struct App {
    backend: Arc<dyn Backend>,
    scanner: Scanner,
    scanning: Option<Receiver<Catalog>>,
    catalog: Option<Arc<Catalog>>,
    view: View,
    visible: Vec<usize>,
    selection: Option<Selection>,
    icons: IconCache,
    dialog: Option<Dialog>,
    notice: Option<String>,
    theme: Theme,
    applied_theme: Option<Theme>,
    choice: Choice,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs() as i64)
}

/// Width below which the tool bar takes two lines, and the side bar and the narrowest list.
const TOOL_BAR_ONE_LINE: f32 = du(195.0);
const SIDE_BAR: f32 = du(58.0);
const LIST_MIN: f32 = du(112.0);
const CARD_MIN: f32 = du(90.0);

fn sorts(text: &Text) -> [(SortKey, &'static str); 4] {
    [
        (SortKey::LastRun, text.sort_last_run),
        (SortKey::Name, text.sort_name),
        (SortKey::Size, text.sort_size),
        (SortKey::Installed, text.sort_installed),
    ]
}

fn origins(text: &Text) -> [(OriginFilter, &'static str); 4] {
    [
        (OriginFilter::All, text.origin_all),
        (OriginFilter::Explicit, text.origin_explicit),
        (OriginFilter::Dependency, text.origin_dependency),
        (OriginFilter::Orphans, text.origin_orphans),
    ]
}

impl App {
    pub fn new(ctx: &egui::Context, backend: Arc<dyn Backend>, scanner: Scanner, choice: Choice) -> Self {
        let theme = match ctx.system_theme() {
            Some(egui::Theme::Light) => Theme::Light,
            _ => Theme::Dark,
        };
        let mut app = Self {
            backend,
            scanner,
            scanning: None,
            catalog: None,
            view: View::default(),
            visible: Vec::new(),
            selection: None,
            icons: IconCache::new(ctx),
            dialog: None,
            notice: None,
            theme,
            applied_theme: None,
            choice,
        };
        app.rescan(ctx);
        app
    }

    fn text(&self) -> &'static Text {
        self.choice.lang.text()
    }

    /// Descriptions of programs come in the language of the scan, so the catalogue is read again.
    fn switch_language(&mut self, ctx: &egui::Context) {
        self.choice.set(self.choice.lang.other());
        self.notice = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.text().app_name.to_owned()));
        self.rescan(ctx);
    }

    fn rescan(&mut self, ctx: &egui::Context) {
        let (sender, receiver) = channel();
        let scanner = Arc::clone(&self.scanner);
        let lang = self.choice.lang;
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = sender.send(scanner(lang));
            ctx.request_repaint();
        });
        self.scanning = Some(receiver);
    }

    fn refilter(&mut self) {
        self.visible = self
            .catalog
            .as_ref()
            .map(|catalog| view::visible(&catalog.items, &self.view))
            .unwrap_or_default();
    }

    fn receive_scan(&mut self) {
        let finished = self
            .scanning
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(catalog) = finished {
            self.catalog = Some(Arc::new(catalog));
            self.scanning = None;
            self.refilter();
            let selected = self.selection.as_ref().map(|selection| selection.id.clone());
            self.selection = None;
            if let Some(id) = selected {
                self.select(&id);
            }
        }
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
        ui.horizontal(|ui| {
            ui.label(styled(text.app_name, Type::AppTitle, palette.text_primary));
            if let Some(catalog) = &self.catalog {
                let total: u64 = catalog.items.iter().filter_map(|item| item.size).sum();
                let line = format!(
                    "{}, {}",
                    (text.records)(catalog.items.len()),
                    format::size(total, text)
                );
                ui.label(styled(line, Type::Subtitle, palette.text_secondary));
            }
            if self.scanning.is_some() {
                ui.label(styled(text.reading_system, Type::Subtitle, palette.pending));
            } else if let Some(notice) = &self.notice {
                ui.label(styled(notice.clone(), Type::Subtitle, palette.success));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (glyph, label) = match self.theme {
                    Theme::Dark => (glyph::SUN, text.light_theme),
                    Theme::Light => (glyph::MOON, text.dark_theme),
                };
                if icon_button(ui, glyph, STANDARD_ROW, label, palette).clicked() {
                    self.theme = match self.theme {
                        Theme::Dark => Theme::Light,
                        Theme::Light => Theme::Dark,
                    };
                }
                if icon_button(ui, text.switch_mark, STANDARD_ROW, text.switch_language, palette).clicked() {
                    self.switch_language(ui.ctx());
                }
                let idle = self.scanning.is_none();
                if action_button(
                    ui,
                    text.refresh,
                    Some(glyph::ARROW_CLOCKWISE),
                    Tone::Plain,
                    idle,
                    palette,
                )
                .clicked()
                {
                    self.notice = None;
                    self.rescan(ui.ctx());
                }
                let width = clamp_width(ui.available_width(), 40.0, 0.5, 120.0).round();
                if search_field(ui, &mut self.view.query, width, text, palette) {
                    self.refilter();
                }
            });
        });
    }

    fn scope_row(&mut self, ui: &mut Ui, glyph: &str, label: &str, scope: Scope, palette: &Palette) {
        let items = self
            .catalog
            .as_ref()
            .map_or(&[][..], |catalog| catalog.items.as_slice());
        let count = self.view.count(items, scope);
        if nav_row(ui, glyph, label, count, self.view.scope == scope, palette).clicked() {
            self.view.scope = scope;
            self.refilter();
        }
    }

    fn source_row(&mut self, ui: &mut Ui, kind: Option<SourceKind>, palette: &Palette) {
        let items = self
            .catalog
            .as_ref()
            .map_or(&[][..], |catalog| catalog.items.as_slice());
        let probe = View {
            source: kind,
            ..self.view.clone()
        };
        let count = probe.count(items, self.view.scope);
        let text = self.text();
        let (icon, label) = kind.map_or((glyph::SQUARES_FOUR, text.all_sources), |kind| {
            (glyph::of_source(kind), (text.source)(kind))
        });
        if nav_row(ui, icon, label, count, self.view.source == kind, palette).clicked() {
            self.view.source = kind;
            self.refilter();
        }
    }

    fn side_bar(&mut self, ui: &mut Ui, palette: &Palette) {
        let text = self.text();
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.scope_row(ui, glyph::SQUARES_FOUR, text.all_programs, Scope::Apps, palette);
            for section in Section::APPS {
                self.scope_row(
                    ui,
                    glyph::of_section(section),
                    (text.section)(section),
                    Scope::One(section),
                    palette,
                );
            }
            nav_heading(ui, text.other, palette);
            for section in Section::OTHER {
                self.scope_row(
                    ui,
                    glyph::of_section(section),
                    (text.section)(section),
                    Scope::One(section),
                    palette,
                );
            }
            self.scope_row(ui, glyph::PACKAGE, text.everything, Scope::Everything, palette);
            nav_heading(ui, text.source_heading, palette);
            self.source_row(ui, None, palette);
            for kind in SourceKind::ALL {
                self.source_row(ui, Some(kind), palette);
            }
        });
    }

    fn sort_controls(&mut self, ui: &mut Ui, palette: &Palette) {
        let text = self.text();
        let sorts = sorts(text);
        let labels = sorts.map(|(_, label)| label);
        let current = sorts
            .iter()
            .position(|(key, _)| *key == self.view.sort)
            .unwrap_or(0);
        ui.label(styled(text.sort_by, Type::Subtitle, palette.text_secondary));
        if let Some(choice) = segmented(ui, &labels, current, palette) {
            self.view.sort = sorts[choice].0;
            self.refilter();
        }
        let (arrow, hint) = match self.view.order {
            Order::Ascending => (glyph::ARROW_UP, text.ascending),
            Order::Descending => (glyph::ARROW_DOWN, text.descending),
        };
        if icon_button(ui, arrow, SEGMENT_HEIGHT, hint, palette).clicked() {
            self.view.order = self.view.order.flipped();
            self.refilter();
        }
    }

    fn origin_controls(&mut self, ui: &mut Ui, palette: &Palette) {
        let origins = origins(self.text());
        let labels = origins.map(|(_, label)| label);
        let current = origins
            .iter()
            .position(|(origin, _)| *origin == self.view.origin)
            .unwrap_or(0);
        if let Some(choice) = segmented(ui, &labels, current, palette) {
            self.view.origin = origins[choice].0;
            self.refilter();
        }
        ui.label(styled(
            self.visible.len().to_string(),
            Type::Subtitle,
            palette.text_secondary,
        ));
    }

    /// Two lines when the card leaves little room. egui cannot wrap a group whose width it learns
    /// only after drawing it, and a row wider than the column stretches the list under the card.
    fn tool_bar(&mut self, ui: &mut Ui, palette: &Palette) {
        if ui.available_width() < TOOL_BAR_ONE_LINE {
            ui.horizontal(|ui| self.sort_controls(ui, palette));
            ui.horizontal(|ui| self.origin_controls(ui, palette));
        } else {
            ui.horizontal(|ui| {
                self.sort_controls(ui, palette);
                ui.add_space(GUTTER);
                self.origin_controls(ui, palette);
            });
        }
    }

    fn list(&mut self, ui: &mut Ui, now: i64, palette: &Palette) {
        let text = self.text();
        let Some(catalog) = self.catalog.clone() else {
            ui.centered_and_justified(|ui| {
                ui.label(weighted(
                    text.reading_system,
                    Type::Body,
                    Weight::Medium,
                    palette.pending,
                ));
            });
            return;
        };
        if self.visible.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(styled(text.nothing_found, Type::Body, palette.text_secondary));
            });
            return;
        }
        let rows = list::Rows {
            items: &catalog.items,
            indices: &self.visible,
            selected: self.selection.as_ref().map(|selection| selection.id.as_str()),
            now,
            text,
        };
        if let Some(id) = list::show(ui, &rows, &mut self.icons, palette) {
            self.select(&id);
        }
    }

    fn card(&mut self, ui: &mut Ui, now: i64, palette: &Palette) {
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        let card = detail::Card {
            item: &item,
            command: self
                .selection
                .as_ref()
                .and_then(|selection| selection.command.as_deref()),
            now,
            text: self.text(),
        };
        match detail::show(ui, &card, &mut self.icons, palette) {
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
            Some(detail::Action::Open(url)) => ui.ctx().open_url(egui::OpenUrl::new_tab(url)),
            Some(detail::Action::Follow(name)) => self.select(&format!("pacman:{name}")),
            None => {}
        }
    }

    /// One frame of the whole window.
    pub fn draw(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let palette = self.theme.palette();
        if self.applied_theme.is_none() {
            theme::install_fonts(&ctx);
        }
        if self.applied_theme != Some(self.theme) {
            theme::apply(&ctx, palette, self.theme == Theme::Dark);
            self.applied_theme = Some(self.theme);
        }
        self.receive_scan();
        self.icons.receive(&ctx);
        let now = unix_now();

        let bar = Frame::new()
            .fill(palette.background)
            .inner_margin(Margin::symmetric(MARGIN_SCREEN as i8, PAD_SMALL as i8));
        Panel::top("header")
            .frame(bar)
            .show(ui, |ui| self.header(ui, palette));

        let side = Frame::new().fill(palette.background);
        Panel::left("sections")
            .frame(side)
            .exact_size(SIDE_BAR)
            .resizable(false)
            .show(ui, |ui| self.side_bar(ui, palette));

        if self.selection.is_some() {
            let card_limit = (ui.available_width() - LIST_MIN).clamp(CARD_MIN, du(190.0));
            let card = Frame::new()
                .fill(palette.surface)
                .inner_margin(Margin::same(MARGIN_SCREEN as i8));
            Panel::right("card")
                .frame(card)
                .default_size(du(115.0))
                .size_range(CARD_MIN..=card_limit)
                .show(ui, |ui| self.card(ui, now, palette));
        }

        let centre = Frame::new().fill(palette.background);
        CentralPanel::default().frame(centre).show(ui, |ui| {
            Frame::new()
                .inner_margin(Margin::symmetric(PAD_CARD as i8, PAD_SMALL as i8))
                .show(ui, |ui| self.tool_bar(ui, palette));
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            self.list(ui, now, palette);
        });

        let exit = self.dialog.as_mut().and_then(|dialog| dialog.show(&ctx, palette));
        match exit {
            Some(Exit::Removed(title)) => {
                self.dialog = None;
                self.selection = None;
                self.notice = Some((self.text().removed)(&title));
                self.rescan(&ctx);
            }
            Some(Exit::Cancelled) => self.dialog = None,
            None => {}
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}
