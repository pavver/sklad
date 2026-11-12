//! The catalogue as a `DataTable` (draft): name, install date, size, last launch.

use eframe::egui::{Id, Ui};

use super::format;
use super::operator::data_table::{Column, DataTable, Direction, Sorting, Width};
use super::operator::feedback::{empty_state, skeleton_lines};
use super::operator::icon;
use super::operator::tokens::{Palette, dim, du};
use super::view::{Order, SortKey, View};
use crate::i18n::Text;
use crate::model::Item;

/// The sort keys in the order of the columns.
const KEYS: [SortKey; 4] = [SortKey::Name, SortKey::Installed, SortKey::Size, SortKey::LastRun];

pub fn sorting_of(view: &View) -> Sorting {
    let column = KEYS.iter().position(|key| *key == view.sort)?;
    let direction = match view.order {
        Order::Ascending => Direction::Ascending,
        Order::Descending => Direction::Descending,
    };
    Some((column, direction))
}

pub fn sort_by(view: &mut View, sorting: Sorting) {
    match sorting {
        Some((column, direction)) => {
            view.sort = KEYS.get(column).copied().unwrap_or(SortKey::Catalogue);
            view.order = match direction {
                Direction::Ascending => Order::Ascending,
                Direction::Descending => Order::Descending,
            };
        }
        None => {
            view.sort = SortKey::Catalogue;
            view.order = Order::Ascending;
        }
    }
}

pub struct Rows<'a> {
    pub items: &'a [Item],
    pub indices: &'a [usize],
    pub selected: Option<&'a str>,
    pub sorting: Sorting,
    pub loading: bool,
    pub now: i64,
    pub text: &'static Text,
}

pub enum Event {
    Select(String),
    Sort(Sorting),
}

fn cell(item: &Item, column: usize, rows: &Rows<'_>) -> String {
    let text = rows.text;
    match column {
        0 => item.title.clone(),
        1 => item
            .installed
            .map_or_else(|| "—".to_owned(), |at| format::date(at, text)),
        2 => item
            .size
            .map_or_else(|| "—".to_owned(), |size| format::size(size, text)),
        _ => format::usage(item.usage, rows.now, text),
    }
}

pub fn show(ui: &mut Ui, rows: &Rows<'_>, palette: &Palette) -> Option<Event> {
    let text = rows.text;
    let columns = [
        Column {
            title: text.column_program,
            width: Width::Share(1.0),
            numeric: false,
            sortable: true,
        },
        Column {
            title: text.sort_installed,
            width: Width::Fixed(32.0),
            numeric: true,
            sortable: true,
        },
        Column {
            title: text.column_size,
            width: Width::Fixed(22.0),
            numeric: true,
            sortable: true,
        },
        Column {
            title: text.column_last_run,
            width: Width::Fixed(40.0),
            numeric: true,
            sortable: true,
        },
    ];
    let selected = rows
        .selected
        .and_then(|id| rows.indices.iter().position(|&index| rows.items[index].id == id));
    let table = DataTable {
        id: Id::unique("catalogue"),
        columns: &columns,
        rows: rows.indices.len(),
        sorting: rows.sorting,
        selected,
        loading: rows.loading,
    };
    let item = |row: usize| &rows.items[rows.indices[row]];
    let events = table.show(
        ui,
        palette,
        |row, column| cell(item(row), column, rows),
        |row| item(row).title.clone(),
        |ui| {
            if rows.loading {
                ui.add_space(du(dim::PAD_CARD));
                ui.horizontal(|ui| {
                    ui.add_space(du(dim::PAD_CARD));
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - du(dim::PAD_CARD));
                        skeleton_lines(ui, 6, palette);
                    });
                });
            } else {
                let rect = ui.max_rect();
                empty_state(ui, rect, icon::INBOX, text.nothing_found, None, true, palette);
            }
        },
    );
    if let Some(sorting) = events.sorting {
        return Some(Event::Sort(sorting));
    }
    events.activated.map(|row| Event::Select(item(row).id.clone()))
}
