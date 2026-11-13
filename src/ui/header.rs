//! The top bar: refresh, title, search and the switches of language and theme.

use eframe::egui::{Id, Ui};

use crate::i18n::Lang;

use super::operator::button::IconButton;
use super::operator::combobox::{self, Combobox};
use super::operator::icon;
use super::operator::layout::{HeaderCentre, header};
use super::operator::search_field::SearchField;
use super::operator::state::result_hold;
use super::operator::theme_toggle::theme_toggle;
use super::operator::tokens::{Palette, dim, du};
use super::{App, format};

/// The width in DU the search in the middle of the header would like.
const SEARCH_WIDTH: f32 = 80.0;
/// The side in DU of the refresh button before the title, a size the specification lists.
const REFRESH_SIZE: f32 = 11.0;

impl App {
    pub(super) fn header(&mut self, ui: &mut Ui, palette: &Palette) {
        let text = self.text();
        // What the list shows, and how much it takes: all of it, or the part the filters and the
        // search leave.
        let subtitle = match &self.catalog {
            Some(_) if self.scanning.is_some() => text.reading_system.to_owned(),
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
        let query_text = &mut self.view.query;
        let search = HeaderCentre {
            width: du(SEARCH_WIDTH),
            content: |ui: &mut Ui| {
                let field = SearchField {
                    id: Id::unique("search"),
                    placeholder: text.search,
                    clear_label: text.clear_search,
                    height: dim::ROW_COMPACT,
                    width: ui.available_width(),
                };
                query = field.show(ui, query_text, palette);
            },
        };
        let refresh_button = |ui: &mut Ui| {
            refresh = IconButton::new(icon::REFRESH, text.refresh)
                .size(REFRESH_SIZE)
                .icon_size(dim::ICON_LARGE)
                .look(look)
                .show(ui, palette)
                .clicked();
        };
        let start = Some(refresh_button);
        header(
            ui,
            text.app_name,
            Some(&subtitle),
            palette,
            start,
            Some(search),
            |ui| {
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
            },
        );
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
}
