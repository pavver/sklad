//! What the list shows: the filters and the order. Plain data, so it is tested without a window.

use std::cmp::Ordering;

use crate::model::{Item, Reason, Section, SourceKind, Usage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Apps,
    Everything,
    One(Section),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginFilter {
    All,
    Explicit,
    Dependency,
    Orphans,
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
    pub scope: Scope,
    pub source: Option<SourceKind>,
    pub origin: OriginFilter,
    pub sort: SortKey,
    pub order: Order,
}

impl Default for View {
    /// Opens on the cleaning view: programs, the ones unused for longest first.
    fn default() -> Self {
        Self {
            query: String::new(),
            scope: Scope::Apps,
            source: None,
            origin: OriginFilter::All,
            sort: SortKey::LastRun,
            order: Order::Ascending,
        }
    }
}

impl Scope {
    pub fn holds(self, item: &Item) -> bool {
        match self {
            Scope::Apps => item.section.is_app(),
            Scope::Everything => true,
            Scope::One(section) => item.section == section,
        }
    }
}

impl OriginFilter {
    pub fn holds(self, item: &Item) -> bool {
        match self {
            OriginFilter::All => true,
            OriginFilter::Explicit => item.reason != Some(Reason::Dependency),
            OriginFilter::Dependency => item.reason == Some(Reason::Dependency),
            OriginFilter::Orphans => item.is_orphan(),
        }
    }
}

impl View {
    fn holds_apart_from_scope(&self, item: &Item, words: &[String]) -> bool {
        self.source.is_none_or(|source| item.source.kind() == source)
            && self.origin.holds(item)
            && words.iter().all(|word| item.haystack.contains(word))
    }

    /// The scope as it applies here. A source of command-line tools holds no windowed programs, so
    /// "all programs" would show it empty while it has entries.
    fn reach(&self, scope: Scope) -> Scope {
        let tools = self.source.is_some_and(SourceKind::holds_only_tools);
        if tools && scope == Scope::Apps {
            Scope::Everything
        } else {
            scope
        }
    }

    fn words(&self) -> Vec<String> {
        self.query
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    }

    /// How many items a scope would show under the other filters: the numbers in the side bar.
    pub fn count(&self, items: &[Item], scope: Scope) -> usize {
        let words = self.words();
        let scope = self.reach(scope);
        items
            .iter()
            .filter(|item| scope.holds(item) && self.holds_apart_from_scope(item, &words))
            .count()
    }
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
    let scope = view.reach(view.scope);
    let mut indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| scope.holds(item) && view.holds_apart_from_scope(item, &words))
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
            groups: Vec::new(),
            depends: Vec::new(),
            required_by: Vec::new(),
            removal: Removal::Pacman,
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
    fn the_cleaning_view_opens_with_what_was_unused_longest() {
        assert_eq!(
            titles(&sample(), &View::default()),
            ["Never", "Stale", "Fresh", "Live"]
        );
    }

    #[test]
    fn newest_first_is_the_exact_reverse_and_console_programs_stay_last() {
        let view = View {
            scope: Scope::Everything,
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
            scope: Scope::Everything,
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
    fn an_orphan_is_a_dependency_nothing_needs() {
        let mut items = sample();
        items[0].reason = Some(Reason::Dependency);
        items[1].reason = Some(Reason::Dependency);
        items[1].required_by = vec!["x".to_owned()];
        let view = View {
            origin: OriginFilter::Orphans,
            ..View::default()
        };
        assert_eq!(titles(&items, &view), ["Fresh"]);
    }

    #[test]
    fn a_source_of_tools_is_not_empty_under_all_programs() {
        let mut tool = item("Rust stable", Section::Console, Usage::NotTracked);
        tool.source = Source::Rustup;
        let mut items = sample();
        items.push(tool);
        let view = View {
            source: Some(SourceKind::Rustup),
            ..View::default()
        };
        assert_eq!(view.count(&items, Scope::Apps), 1);
        assert_eq!(titles(&items, &view), ["Rust stable"]);
        assert_eq!(view.count(&items, Scope::One(Section::Internet)), 0);
        assert!(!titles(&items, &View::default()).contains(&"Rust stable".to_owned()));
    }

    #[test]
    fn the_numbers_of_the_side_bar_follow_the_other_filters() {
        let items = sample();
        let view = View {
            query: "fresh".to_owned(),
            ..View::default()
        };
        assert_eq!(view.count(&items, Scope::One(Section::Internet)), 1);
        assert_eq!(view.count(&items, Scope::One(Section::Games)), 0);
    }
}
