//! The side bar: the filters by section, source and origin, with their icons and names.

use std::path::PathBuf;

use eframe::egui::{Id, ScrollArea, Ui};

use crate::i18n::Text;
use crate::model::{PackageSystem, Section, SourceKind};

use super::operator::checkbox_group::{self, CheckboxGroup, GroupItem, IconSize, TileIcon};
use super::operator::collapsible::{self, Collapsible};
use super::operator::content::Picture;
use super::operator::icon::{self, Icon};
use super::operator::tokens::Palette;
use super::view::{Facet, Origin};
use super::{App, view};

/// The largest number a tile of the side bar shows in full; a catalogue rarely holds more.
const COUNT_MAX: usize = 99_999;

fn origin_icon(origin: Origin) -> Icon {
    match origin {
        Origin::Explicit => icon::PERSON,
        Origin::Needed => icon::LINK,
        Origin::Orphan => icon::LINK_OFF,
    }
}

/// The names of the icon of a filter in the icon themes. Only sections take theirs from the
/// theme: sources show logos sklad carries, and origins Material Symbols.
fn theme_names(facet: Facet) -> &'static [&'static str] {
    match facet {
        Facet::Section(section) => section.theme_icons(),
        Facet::Source(_) | Facet::Origin(_) => &[],
    }
}

/// The icon and the name of a filter, as its tile and its chip say it.
pub(super) fn facet_label(facet: Facet, text: &Text, system: Option<PackageSystem>) -> (Icon, &'static str) {
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
        Facet::Source(source) => (source_icon(source, system), (text.source)(source)),
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

/// The logo of a source, the same on every system; a glyph where the source has no logo of its own.
fn source_icon(source: SourceKind, system: Option<PackageSystem>) -> Icon {
    match source {
        SourceKind::Repo => match system {
            Some(PackageSystem::Pacman) => icon::LOGO_ARCHLINUX,
            Some(PackageSystem::Dpkg) => icon::LOGO_DEBIAN,
            None => icon::INVENTORY_2,
        },
        SourceKind::Aur => icon::GROUPS,
        SourceKind::Flatpak => icon::LOGO_FLATPAK,
        SourceKind::Steam => icon::LOGO_STEAM,
        SourceKind::Cargo | SourceKind::Rustup => icon::LOGO_RUST,
        SourceKind::Node => icon::LOGO_NODEJS,
        SourceKind::Python => icon::LOGO_PYTHON,
        SourceKind::LocalDeb => icon::PACKAGE_2,
        SourceKind::Docker => icon::LOGO_DOCKER,
        SourceKind::Manual => icon::FOLDER,
    }
}

impl App {
    /// One group of filters: a `Collapsible` holding a `CheckboxGroup`, each tile with the number
    /// choosing it would show. The header counts the filters of the group that are on, so a
    /// closed group still says it filters. `key` names the group whatever the language.
    fn facets(
        &mut self,
        ui: &mut Ui,
        key: &str,
        title: &str,
        facets: &[Facet],
        palette: &Palette,
    ) {
        if facets.is_empty() {
            return;
        }
        let text = self.text();
        let items = self
            .catalog
            .as_ref()
            .map_or(&[][..], |catalog| catalog.items.as_slice());
        let theme = self.catalog.as_ref().map(|catalog| &catalog.theme_icons);
        let system = self.catalog.as_ref().and_then(|catalog| catalog.system);
        let pictures = &mut self.pictures;
        let tiles: Vec<GroupItem<'_>> = facets
            .iter()
            .map(|&facet| {
                let (glyph, name) = facet_label(facet, text, system);
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

    /// The side bar while the catalogue loads: the four groups in the shape they will take.
    fn side_bar_skeleton(ui: &mut Ui, palette: &Palette) {
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (seed, tiles) in [6, 3, 3, 3].into_iter().enumerate() {
                collapsible::skeleton(ui, seed, palette, |ui| {
                    checkbox_group::skeleton(ui, tiles, IconSize::Large, palette);
                });
            }
        });
    }

    pub(super) fn side_bar(&mut self, ui: &mut Ui, palette: &Palette) {
        if self.loading() {
            Self::side_bar_skeleton(ui, palette);
            return;
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiles of one group are told apart by their icons, so no two share one.
    fn distinct(icons: Vec<Icon>) -> bool {
        let mut names: Vec<&str> = icons.iter().map(|icon| icon.name()).collect();
        names.sort_unstable();
        names.dedup();
        names.len() == icons.len()
    }

    #[test]
    fn every_section_has_its_own_glyph_and_name_in_the_theme() {
        let sections: Vec<Section> = Section::APPS.iter().chain(&Section::OTHER).copied().collect();
        assert!(distinct(
            sections.iter().map(|&section| section_icon(section)).collect()
        ));
        for section in sections {
            assert!(!theme_names(Facet::Section(section)).is_empty(), "{section:?}");
        }
    }

    #[test]
    fn sources_show_their_logos_and_the_repositories_that_of_the_distribution() {
        let pacman = Some(PackageSystem::Pacman);
        assert_eq!(source_icon(SourceKind::Repo, pacman), icon::LOGO_ARCHLINUX);
        assert_eq!(
            source_icon(SourceKind::Repo, Some(PackageSystem::Dpkg)),
            icon::LOGO_DEBIAN
        );
        assert_eq!(source_icon(SourceKind::Repo, None), icon::INVENTORY_2);
        // Cargo and Rustup both belong to Rust and share its logo; every other source has its own.
        let others: Vec<Icon> = SourceKind::ALL
            .iter()
            .filter(|&&source| source != SourceKind::Rustup)
            .map(|&source| source_icon(source, pacman))
            .collect();
        assert!(distinct(others));
        assert_eq!(source_icon(SourceKind::Rustup, pacman), icon::LOGO_RUST);
        assert!(
            theme_names(Facet::Source(SourceKind::Steam)).is_empty(),
            "sources never use the theme"
        );
        assert!(theme_names(Facet::Origin(Origin::Orphan)).is_empty());
    }
}
