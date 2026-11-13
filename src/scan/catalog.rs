//! The catalogue: everything the scan found, and how a newer scan updates it in place.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::model::{Item, PackageSystem};
use crate::plan::PackageDb;

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub items: Vec<Item>,
    pub db: PackageDb,
    /// The package system of the distribution, `None` when there is none the catalogue reads.
    pub system: Option<PackageSystem>,
    /// The icons of sections and sources found in the icon themes, by name.
    pub theme_icons: HashMap<&'static str, PathBuf>,
}

impl Catalog {
    /// The catalogue without the items `gone` names by id: what a removal that succeeded took.
    pub fn without(&self, gone: &[String]) -> Self {
        let gone: HashSet<&str> = gone.iter().map(String::as_str).collect();
        Self {
            items: self
                .items
                .iter()
                .filter(|item| !gone.contains(item.id.as_str()))
                .cloned()
                .collect(),
            ..self.clone()
        }
    }

    /// `newer` laid over this catalogue, item by item: an item keeps its place and takes what
    /// the newer scan says of it, an item the newer scan lacks leaves, and a new one comes last.
    /// Rows the sort cannot tell apart so keep their order.
    pub fn updated(&self, newer: Self) -> Self {
        let mut fresh: HashMap<String, Item> = HashMap::new();
        let mut order = Vec::with_capacity(newer.items.len());
        for item in newer.items {
            order.push(item.id.clone());
            fresh.insert(item.id.clone(), item);
        }
        let mut items: Vec<Item> = self
            .items
            .iter()
            .filter_map(|item| fresh.remove(&item.id))
            .collect();
        items.extend(order.iter().filter_map(|id| fresh.remove(id)));
        Self { items, ..newer }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Removal, Source};
    use crate::scan::blank;

    fn catalog(items: &[(&str, &str)]) -> Catalog {
        Catalog {
            items: items
                .iter()
                .map(|&(id, version)| {
                    let mut item = blank(
                        id.to_owned(),
                        id.to_owned(),
                        Source::Cargo,
                        Removal::Cargo { name: id.to_owned() },
                    );
                    item.version = version.to_owned();
                    item
                })
                .collect(),
            ..Catalog::default()
        }
    }

    fn shown(catalog: &Catalog) -> Vec<(String, String)> {
        catalog
            .items
            .iter()
            .map(|item| (item.id.clone(), item.version.clone()))
            .collect()
    }

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|&(id, version)| (id.to_owned(), version.to_owned()))
            .collect()
    }

    #[test]
    fn what_was_removed_leaves_and_the_rest_stays_in_order() {
        let old = catalog(&[("a", "1"), ("b", "1"), ("c", "1")]);
        let left = old.without(&["b".to_owned(), "missing".to_owned()]);
        assert_eq!(shown(&left), pairs(&[("a", "1"), ("c", "1")]));
    }

    #[test]
    fn a_newer_scan_changes_items_in_their_places() {
        let old = catalog(&[("a", "1"), ("b", "1"), ("c", "1"), ("d", "1")]);
        let newer = catalog(&[("e", "1"), ("d", "1"), ("c", "2"), ("a", "1")]);
        assert_eq!(
            shown(&old.updated(newer)),
            pairs(&[("a", "1"), ("c", "2"), ("d", "1"), ("e", "1")]),
            "b left, c changed in its place, e came last"
        );
    }

    #[test]
    fn the_rest_of_a_newer_scan_is_taken_whole() {
        let old = catalog(&[("a", "1")]);
        let newer = Catalog {
            system: Some(PackageSystem::Dpkg),
            ..catalog(&[("a", "1")])
        };
        assert_eq!(old.updated(newer).system, Some(PackageSystem::Dpkg));
    }
}
