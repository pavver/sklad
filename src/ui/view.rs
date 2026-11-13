//! What the list shows: the filters and the order. Plain data, so it is tested without a window.
//!
//! The filters are facets in three groups: sections, origins and sources. Within a group the
//! chosen facets add up; between groups they narrow; a group with none chosen does not filter.

use std::cmp::Ordering;

use crate::model::{Item, PackageSystem, Reason, Section, SourceKind, Usage};

/// Why an item is on the system, in three parts that never overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Installed by the person, or by a manager that has no dependencies.
    Explicit,
    /// A dependency something still needs.
    Needed,
    /// A dependency nothing needs any more.
    Orphan,
}

impl Origin {
    pub const ALL: [Origin; 3] = [Origin::Explicit, Origin::Needed, Origin::Orphan];

    pub fn of(item: &Item) -> Origin {
        if item.reason != Some(Reason::Dependency) {
            Origin::Explicit
        } else if item.is_orphan() {
            Origin::Orphan
        } else {
            Origin::Needed
        }
    }
}

/// One filter of the side bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Facet {
    Section(Section),
    Origin(Origin),
    Source(SourceKind),
}

/// The groups of facets: within one they add up, between them they narrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Section,
    Origin,
    Source,
}

const GROUPS: [Group; 3] = [Group::Section, Group::Origin, Group::Source];

impl Facet {
    fn group(self) -> Group {
        match self {
            Facet::Section(_) => Group::Section,
            Facet::Origin(_) => Group::Origin,
            Facet::Source(_) => Group::Source,
        }
    }

    pub fn holds(self, item: &Item) -> bool {
        match self {
            Facet::Section(section) => item.section == section,
            Facet::Origin(origin) => Origin::of(item) == origin,
            Facet::Source(source) => item.source.kind() == source,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    LastRun,
    Name,
    Size,
    Installed,
    /// The order of the catalogue itself.
    Catalogue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub query: String,
    /// The chosen facets, in the order they were chosen.
    filters: Vec<Facet>,
    pub sort: SortKey,
    pub order: Order,
}

impl Default for View {
    /// Opens on everything, the entries unused for longest first.
    fn default() -> Self {
        Self {
            query: String::new(),
            filters: Vec::new(),
            sort: SortKey::LastRun,
            order: Order::Ascending,
        }
    }
}

impl View {
    /// A view with these facets chosen.
    pub fn with(filters: &[Facet]) -> Self {
        Self {
            filters: filters.to_vec(),
            ..Self::default()
        }
    }

    pub fn filters(&self) -> &[Facet] {
        &self.filters
    }

    pub fn is_on(&self, facet: Facet) -> bool {
        self.filters.contains(&facet)
    }

    pub fn toggle(&mut self, facet: Facet) {
        if self.is_on(facet) {
            self.filters.retain(|on| *on != facet);
        } else {
            self.filters.push(facet);
        }
    }

    pub fn clear(&mut self) {
        self.filters.clear();
    }

    fn words(&self) -> Vec<String> {
        self.query
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    }

    /// Whether the item passes the search and every group but `skip`.
    fn holds(&self, item: &Item, words: &[String], skip: Option<Group>) -> bool {
        let groups = GROUPS.iter().filter(|group| Some(**group) != skip).all(|&group| {
            let mut chosen = self
                .filters
                .iter()
                .filter(|facet| facet.group() == group)
                .peekable();
            chosen.peek().is_none() || chosen.any(|facet| facet.holds(item))
        });
        groups && words.iter().all(|word| item.haystack.contains(word))
    }

    /// How many items choosing `facet` would show, under the search and the other groups: the
    /// number on its tile.
    pub fn count(&self, items: &[Item], facet: Facet) -> usize {
        let words = self.words();
        items
            .iter()
            .filter(|item| facet.holds(item) && self.holds(item, &words, Some(facet.group())))
            .count()
    }
}

/// The sections the side bar offers: those with an entry, and the chosen ones, so a filter
/// does not vanish under the pointer when its last entry is removed.
pub fn offered_sections(items: &[Item], sections: &[Section], view: &View) -> Vec<Section> {
    sections
        .iter()
        .copied()
        .filter(|section| {
            view.is_on(Facet::Section(*section)) || items.iter().any(|item| item.section == *section)
        })
        .collect()
}

/// The sources the side bar offers: those that can exist on this system and hold at least one
/// entry, so the AUR never shows on Debian and an empty Steam never shows at all. Chosen ones
/// stay, as sections do.
pub fn offered_sources(items: &[Item], system: Option<PackageSystem>, view: &View) -> Vec<SourceKind> {
    SourceKind::ALL
        .into_iter()
        .filter(|kind| {
            let present = items.iter().any(|item| item.source.kind() == *kind);
            view.is_on(Facet::Source(*kind)) || (kind.fits(system) && present)
        })
        .collect()
}

/// When the last launch happened, for ordering. `None` means the question does not apply.
fn run_rank(usage: Usage) -> Option<i64> {
    match usage {
        Usage::Running => Some(i64::MAX),
        Usage::Last { at, .. } => Some(at),
        Usage::NoRecord => Some(i64::MIN),
        Usage::NotTracked => None,
    }
}

/// Compares two optional values; a missing one goes last in either direction.
fn missing_last<T: Ord>(left: Option<T>, right: Option<T>, order: Order) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => match order {
            Order::Ascending => left.cmp(&right),
            Order::Descending => right.cmp(&left),
        },
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare(left: &Item, right: &Item, view: &View) -> Ordering {
    let order = view.order;
    let primary = match view.sort {
        SortKey::Catalogue => return Ordering::Equal,
        SortKey::LastRun => missing_last(run_rank(left.usage), run_rank(right.usage), order),
        SortKey::Size => missing_last(left.size, right.size, order),
        SortKey::Installed => missing_last(left.installed, right.installed, order),
        SortKey::Name => Ordering::Equal,
    };
    let by_name = left.title.to_lowercase().cmp(&right.title.to_lowercase());
    let by_name = match (view.sort, order) {
        (SortKey::Name, Order::Descending) => by_name.reverse(),
        _ => by_name,
    };
    primary.then(by_name).then_with(|| left.id.cmp(&right.id))
}

/// Indices of the items to show, in display order.
pub fn visible(items: &[Item], view: &View) -> Vec<usize> {
    let words = view.words();
    let mut indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| view.holds(item, &words, None))
        .map(|(index, _)| index)
        .collect();
    indices.sort_by(|&left, &right| compare(&items[left], &items[right], view));
    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Evidence, Removal, Source};

    fn item(title: &str, section: Section, usage: Usage) -> Item {
        Item {
            id: title.to_owned(),
            title: title.to_owned(),
            package: title.to_lowercase(),
            version: String::new(),
            summary: String::new(),
            description: String::new(),
            licenses: Vec::new(),
            homepage: None,
            developer: None,
            source: Source::Repo("extra".to_owned()),
            section,
            reason: Some(Reason::Explicit),
            size: Some(1),
            installed: None,
            usage,
            icon: None,
            launchers: Vec::new(),
            launch: None,
            docker: None,
            groups: Vec::new(),
            depends: Vec::new(),
            required_by: Vec::new(),
            removal: Removal::Package(PackageSystem::Pacman),
            warning: None,
            haystack: String::new(),
        }
        .seal()
    }

    fn last(at: i64) -> Usage {
        Usage::Last {
            at,
            by: Evidence::Plasma,
        }
    }

    fn titles(items: &[Item], view: &View) -> Vec<String> {
        visible(items, view)
            .iter()
            .map(|&i| items[i].title.clone())
            .collect()
    }

    fn sample() -> Vec<Item> {
        vec![
            item("Fresh", Section::Internet, last(900)),
            item("Stale", Section::Internet, last(100)),
            item("Never", Section::Games, Usage::NoRecord),
            item("Live", Section::Office, Usage::Running),
            item("tool", Section::Console, Usage::NotTracked),
        ]
    }

    #[test]
    fn everything_opens_with_what_was_unused_longest_and_untracked_entries_last() {
        assert_eq!(
            titles(&sample(), &View::default()),
            ["Never", "Stale", "Fresh", "Live", "tool"]
        );
    }

    #[test]
    fn newest_first_is_the_exact_reverse_and_console_programs_stay_last() {
        let view = View {
            order: Order::Descending,
            ..View::default()
        };
        assert_eq!(
            titles(&sample(), &view),
            ["Live", "Fresh", "Stale", "Never", "tool"]
        );
    }

    #[test]
    fn a_program_with_no_size_goes_last_in_either_order() {
        let mut items = sample();
        items[0].size = None;
        items[1].size = Some(50);
        for order in [Order::Ascending, Order::Descending] {
            let view = View {
                sort: SortKey::Size,
                order,
                ..View::default()
            };
            assert_eq!(titles(&items, &view).last().map(String::as_str), Some("Fresh"));
        }
    }

    #[test]
    fn the_original_order_is_the_order_of_the_catalogue() {
        let view = View {
            sort: SortKey::Catalogue,
            ..View::default()
        };
        assert_eq!(
            titles(&sample(), &view),
            ["Fresh", "Stale", "Never", "Live", "tool"]
        );
    }

    #[test]
    fn search_needs_every_word_in_any_case() {
        let mut items = sample();
        items[0].summary = "Веб Переглядач".to_owned();
        items[0] = items[0].clone().seal();
        let view = View {
            query: "ПЕРЕГЛЯДАЧ fresh".to_owned(),
            ..View::default()
        };
        assert_eq!(titles(&items, &view), ["Fresh"]);
    }

    #[test]
    fn the_three_origins_never_overlap() {
        let mut items = sample();
        items[0].reason = Some(Reason::Dependency);
        items[1].reason = Some(Reason::Dependency);
        items[1].required_by = vec!["x".to_owned()];
        let only = |origin| titles(&items, &View::with(&[Facet::Origin(origin)]));
        assert_eq!(only(Origin::Orphan), ["Fresh"]);
        assert_eq!(only(Origin::Needed), ["Stale"]);
        assert_eq!(only(Origin::Explicit), ["Never", "Live", "tool"]);
    }

    #[test]
    fn facets_add_up_within_a_group_and_narrow_between_groups() {
        let mut items = sample();
        items[2].source = Source::Steam;
        let games_or_web = View::with(&[Facet::Section(Section::Internet), Facet::Section(Section::Games)]);
        assert_eq!(titles(&items, &games_or_web), ["Never", "Stale", "Fresh"]);
        let mut from_steam = games_or_web.clone();
        from_steam.toggle(Facet::Source(SourceKind::Steam));
        assert_eq!(titles(&items, &from_steam), ["Never"]);
        from_steam.toggle(Facet::Source(SourceKind::Steam));
        assert_eq!(from_steam, games_or_web, "a second click takes the facet off");
    }

    #[test]
    fn a_number_says_what_choosing_its_facet_would_show() {
        let items = sample();
        let view = View {
            query: "fresh".to_owned(),
            ..View::default()
        };
        assert_eq!(view.count(&items, Facet::Section(Section::Internet)), 1);
        assert_eq!(view.count(&items, Facet::Section(Section::Games)), 0);
        let internet = View::with(&[Facet::Section(Section::Internet)]);
        assert_eq!(
            internet.count(&items, Facet::Section(Section::Games)),
            1,
            "a facet of the same group is counted as if it were added"
        );
        assert_eq!(internet.count(&items, Facet::Origin(Origin::Explicit)), 2);
    }

    fn from(source: Source) -> Item {
        Item {
            source,
            ..item("x", Section::Utilities, Usage::NotTracked)
        }
    }

    #[test]
    fn the_side_bar_offers_the_sources_this_system_can_hold_and_holds() {
        let items = [
            from(Source::Repo("extra".to_owned())),
            from(Source::Aur),
            from(Source::Flatpak),
        ];
        let none = View::default();
        let arch = offered_sources(&items, Some(PackageSystem::Pacman), &none);
        assert_eq!(arch, [SourceKind::Repo, SourceKind::Aur, SourceKind::Flatpak]);
        let debian = offered_sources(&items, Some(PackageSystem::Dpkg), &none);
        assert_eq!(
            debian,
            [SourceKind::Repo, SourceKind::Flatpak],
            "no AUR outside Arch"
        );
        let bare = offered_sources(&items, None, &none);
        assert_eq!(
            bare,
            [SourceKind::Flatpak],
            "no repositories without a package system"
        );
    }

    #[test]
    fn a_chosen_filter_stays_after_its_last_entry_is_gone() {
        let items = [from(Source::Flatpak)];
        let view = View::with(&[Facet::Source(SourceKind::Steam), Facet::Section(Section::Games)]);
        let offered = offered_sources(&items, Some(PackageSystem::Pacman), &view);
        assert_eq!(offered, [SourceKind::Flatpak, SourceKind::Steam]);
        let sections = offered_sections(&items, &Section::APPS, &view);
        assert_eq!(sections, [Section::Games, Section::Utilities]);
    }
}
