//! The catalogue: the bar of filters, the list and the card of the selected item.

use eframe::egui::{self, Id, ScrollArea, Ui};

use super::operator::content::{ChipColor, chip, input_chip};
use super::operator::layout::toolbar;
use super::operator::split_pane::{Pane, SplitPane};
use super::operator::state::{Look, result_hold};
use super::operator::tokens::{Palette, dim, du};
use super::side_bar::facet_label;
use super::{App, CARD, detail, list};

/// The narrowest the list and the card get, in DU.
const PANE_MIN: f32 = 90.0;

impl App {
    /// The chosen filters as chips, each with its remove button, and a chip at the end that
    /// takes them all off. Nothing when no filter is on.
    fn filter_bar(&mut self, ui: &mut Ui, palette: &Palette) {
        if self.view.filters().is_empty() || self.loading() {
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
                                let (_, name) = facet_label(facet, text, None);
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
        // While the catalogue is read anew, the rows of the old one give way to the skeleton; while
        // it is read quietly, they stay under the loading bar.
        let indices: &[usize] = if self.loading() { &[] } else { &self.visible };
        let rows = list::Rows {
            items,
            indices,
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
        if let Some(action) = detail::show(ui, &card, &mut self.pictures, palette) {
            self.act(ui.ctx(), item, action, time);
        }
    }

    /// The tool bar over the list, and the card beside it when something is selected.
    pub(super) fn catalogue(&mut self, ui: &mut Ui, now: i64, palette: &Palette) {
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
}
