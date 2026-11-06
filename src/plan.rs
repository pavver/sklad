//! Which packages leave the system when one is removed, worked out on the package graph.
//!
//! The list is computed here rather than by `pacman -Rs` because of metapackages: a metapackage goes
//! away with the program, and its other dependencies must stay. Pacman cannot be told that in one
//! call, and `--print` cannot simulate the `-D --asexplicit` that has to come first. So the exact
//! list is computed, shown, verified by `pacman -Rp` and then passed to pacman by name.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::model::Reason;

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub depends: Vec<String>,
    pub provides: Vec<String>,
    pub reason: Reason,
    /// Holds no files of its own: it exists to pull other packages in.
    pub meta: bool,
    pub size: u64,
}

#[derive(Clone, Debug, Default)]
pub struct PackageDb {
    nodes: Vec<Node>,
    by_name: HashMap<String, usize>,
    providers: HashMap<String, Vec<usize>>,
    needed_by: Vec<Vec<usize>>,
}

/// What to do about packages that cannot work without the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Refuse: they stay, so the target stays.
    Keep,
    /// They go together with the target.
    WithDependents,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PacmanPlan {
    /// The exact list handed to pacman, sorted.
    pub remove: Vec<String>,
    /// Metapackages that go in order to free the target.
    pub metas: Vec<String>,
    /// Former dependencies of those metapackages, marked as installed on purpose so they stay.
    pub keep: Vec<String>,
    /// Programs that need the target. Not empty means the plan cannot run in `Mode::Keep`.
    pub blockers: Vec<String>,
    /// Packages of the list without which the system may not start.
    pub critical: Vec<String>,
    pub freed: u64,
}

impl PacmanPlan {
    pub fn is_blocked(&self) -> bool {
        !self.blockers.is_empty()
    }
}

/// Strips a version constraint or a description: `foo>=1.2` -> `foo`, `foo: for bar` -> `foo`.
pub fn bare_name(spec: &str) -> &str {
    let end = spec.find(['<', '>', '=', ':']).unwrap_or(spec.len());
    spec[..end].trim()
}

/// Keeps an exact version, which tells library versions apart, and cuts everything else:
/// `lib.so=2-64` stays, `foo>=1.2` -> `foo`, `foo: for bar` -> `foo`.
pub fn exact_spec(spec: &str) -> &str {
    let end = spec.find(['<', '>', ':']).unwrap_or(spec.len());
    spec[..end].trim()
}

/// Packages whose absence can leave the machine unable to boot or to reach a desktop.
pub fn is_critical(name: &str) -> bool {
    const EXACT: [&str; 23] = [
        "base",
        "bash",
        "coreutils",
        "dbus",
        "dracut",
        "filesystem",
        "glibc",
        "grub",
        "kwin",
        "linux",
        "mkinitcpio",
        "networkmanager",
        "pacman",
        "pipewire",
        "plasma-desktop",
        "plasma-workspace",
        "polkit",
        "sddm",
        "sudo",
        "systemd",
        "systemd-libs",
        "util-linux",
        "wireplumber",
    ];
    const PREFIXES: [&str; 4] = ["linux-", "zfs", "nvidia", "efibootmgr"];
    EXACT.contains(&name) || PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

impl PackageDb {
    pub fn new(nodes: Vec<Node>) -> Self {
        let mut by_name = HashMap::new();
        let mut providers: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, node) in nodes.iter().enumerate() {
            by_name.insert(node.name.clone(), index);
            providers.entry(node.name.clone()).or_default().push(index);
            for provided in &node.provides {
                for key in [provided.as_str(), bare_name(provided)] {
                    let list = providers.entry(key.to_owned()).or_default();
                    if !list.contains(&index) {
                        list.push(index);
                    }
                }
            }
        }
        let mut needed_by = vec![Vec::new(); nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            for dep in &node.depends {
                let exact = providers.get(dep).or_else(|| providers.get(bare_name(dep)));
                for &provider in exact.map_or(&[][..], Vec::as_slice) {
                    if provider != index && !needed_by[provider].contains(&index) {
                        needed_by[provider].push(index);
                    }
                }
            }
        }
        Self {
            nodes,
            by_name,
            providers,
            needed_by,
        }
    }

    /// The packages satisfying a dependency: by the exact version when one provides it.
    fn providers_of(&self, dep: &str) -> &[usize] {
        self.providers
            .get(dep)
            .or_else(|| self.providers.get(bare_name(dep)))
            .map_or(&[], Vec::as_slice)
    }

    /// A dependency is lost only when every package that satisfies it is gone: removing one of
    /// several fonts does not break what asks for `ttf-font`.
    fn loses_dependency(&self, index: usize, gone: &HashSet<usize>) -> bool {
        self.nodes[index].depends.iter().any(|dep| {
            let providers = self.providers_of(dep);
            !providers.is_empty() && providers.iter().all(|provider| gone.contains(provider))
        })
    }

    /// Everything that needs the package, by name, sorted.
    pub fn required_by(&self, name: &str) -> Vec<String> {
        let Some(&index) = self.by_name.get(name) else {
            return Vec::new();
        };
        let mut names: Vec<String> = self.needed_by[index]
            .iter()
            .map(|&other| self.nodes[other].name.clone())
            .collect();
        names.sort();
        names
    }

    pub fn plan(&self, target: &str, mode: Mode) -> Option<PacmanPlan> {
        let &start = self.by_name.get(target)?;
        let mut gone = HashSet::from([start]);
        let mut metas = BTreeSet::new();
        let mut blockers = BTreeSet::new();

        // Walk outwards from the target. A metapackage is never a reason to keep a program.
        let mut frontier = vec![start];
        while let Some(current) = frontier.pop() {
            for &dependent in &self.needed_by[current] {
                if gone.contains(&dependent) || !self.loses_dependency(dependent, &gone) {
                    continue;
                }
                let node = &self.nodes[dependent];
                if node.meta {
                    metas.insert(dependent);
                } else if mode == Mode::Keep {
                    blockers.insert(node.name.clone());
                    continue;
                }
                gone.insert(dependent);
                frontier.push(dependent);
            }
        }

        let keep = self.siblings_to_keep(&metas, &gone);
        if blockers.is_empty() {
            self.add_unneeded(&mut gone, &keep);
        }

        let mut remove: Vec<String> = gone.iter().map(|&i| self.nodes[i].name.clone()).collect();
        remove.sort();
        let critical = remove.iter().filter(|name| is_critical(name)).cloned().collect();
        let mut keep_names: Vec<String> = keep.iter().map(|&i| self.nodes[i].name.clone()).collect();
        keep_names.sort();
        Some(PacmanPlan {
            freed: gone.iter().map(|&i| self.nodes[i].size).sum(),
            metas: metas.iter().map(|&i| self.nodes[i].name.clone()).collect(),
            keep: keep_names,
            blockers: blockers.into_iter().collect(),
            critical,
            remove,
        })
    }

    fn siblings_to_keep(&self, metas: &BTreeSet<usize>, gone: &HashSet<usize>) -> BTreeSet<usize> {
        let mut keep = BTreeSet::new();
        for &meta in metas {
            for dep in &self.nodes[meta].depends {
                for &provider in self.providers_of(dep) {
                    let node = &self.nodes[provider];
                    if !gone.contains(&provider) && node.reason == Reason::Dependency {
                        keep.insert(provider);
                    }
                }
            }
        }
        keep
    }

    /// The same rule as `pacman -Rs`: a pulled-in dependency goes when nothing that stays uses it.
    /// "Uses" counts a dependency that another package could also satisfy, because the system was
    /// left working with this one. Starts from everything reachable and drops what is held from
    /// outside until nothing changes, which also frees rings of packages needing each other.
    fn add_unneeded(&self, gone: &mut HashSet<usize>, keep: &BTreeSet<usize>) {
        let mut candidates = HashSet::new();
        let mut frontier: Vec<usize> = gone.iter().copied().collect();
        while let Some(current) = frontier.pop() {
            for dep in &self.nodes[current].depends {
                for &provider in self.providers_of(dep) {
                    let pulled_in = self.nodes[provider].reason == Reason::Dependency;
                    if pulled_in
                        && !gone.contains(&provider)
                        && !keep.contains(&provider)
                        && candidates.insert(provider)
                    {
                        frontier.push(provider);
                    }
                }
            }
        }
        loop {
            let held: Vec<usize> = candidates
                .iter()
                .copied()
                .filter(|candidate| {
                    self.needed_by[*candidate]
                        .iter()
                        .any(|user| !gone.contains(user) && !candidates.contains(user))
                })
                .collect();
            if held.is_empty() {
                break;
            }
            for candidate in held {
                candidates.remove(&candidate);
            }
        }
        gone.extend(candidates);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, depends: &[&str], reason: Reason) -> Node {
        Node {
            name: name.to_owned(),
            depends: depends.iter().map(|dep| (*dep).to_owned()).collect(),
            provides: Vec::new(),
            reason,
            meta: false,
            size: 10,
        }
    }

    fn meta(name: &str, depends: &[&str]) -> Node {
        Node {
            meta: true,
            ..node(name, depends, Reason::Explicit)
        }
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_program_goes_with_the_dependency_only_it_needed() {
        let db = PackageDb::new(vec![
            node("app", &["libonly", "libshared"], Reason::Explicit),
            node("other", &["libshared"], Reason::Explicit),
            node("libonly", &[], Reason::Dependency),
            node("libshared", &[], Reason::Dependency),
        ]);
        let plan = db.plan("app", Mode::Keep);
        assert_eq!(plan.map(|p| p.remove), Some(names(&["app", "libonly"])));
    }

    #[test]
    fn a_dependency_installed_on_purpose_stays() {
        let db = PackageDb::new(vec![
            node("app", &["tool"], Reason::Explicit),
            node("tool", &[], Reason::Explicit),
        ]);
        let plan = db.plan("app", Mode::Keep);
        assert_eq!(plan.map(|p| p.remove), Some(names(&["app"])));
    }

    #[test]
    fn a_program_out_of_a_metapackage_leaves_its_siblings_installed() {
        let db = PackageDb::new(vec![
            meta("desktop-meta", &["player", "editor", "viewer"]),
            node("player", &["libplayer"], Reason::Dependency),
            node("editor", &[], Reason::Dependency),
            node("viewer", &[], Reason::Explicit),
            node("libplayer", &[], Reason::Dependency),
        ]);
        let plan = db.plan("player", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.remove, names(&["desktop-meta", "libplayer", "player"]));
        assert_eq!(plan.metas, names(&["desktop-meta"]));
        assert_eq!(
            plan.keep,
            names(&["editor"]),
            "the explicit sibling needs no marking"
        );
        assert!(!plan.is_blocked());
    }

    #[test]
    fn a_metapackage_inside_a_metapackage_is_walked_through() {
        let db = PackageDb::new(vec![
            meta("outer-meta", &["inner-meta", "kept"]),
            meta("inner-meta", &["target"]),
            node("target", &[], Reason::Dependency),
            node("kept", &[], Reason::Dependency),
        ]);
        let plan = db.plan("target", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.remove, names(&["inner-meta", "outer-meta", "target"]));
        assert_eq!(plan.keep, names(&["kept"]));
    }

    #[test]
    fn a_real_program_that_needs_the_target_blocks_it() {
        let db = PackageDb::new(vec![
            node("lib", &[], Reason::Explicit),
            node("app", &["lib"], Reason::Explicit),
        ]);
        let plan = db.plan("lib", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.blockers, names(&["app"]));
        assert!(plan.is_blocked());
    }

    #[test]
    fn removing_with_dependents_takes_the_whole_chain() {
        let db = PackageDb::new(vec![
            node("lib", &[], Reason::Explicit),
            node("app", &["lib"], Reason::Explicit),
            node("plugin", &["app"], Reason::Explicit),
            node("bystander", &[], Reason::Explicit),
        ]);
        let plan = db.plan("lib", Mode::WithDependents).unwrap_or_default();
        assert_eq!(plan.remove, names(&["app", "lib", "plugin"]));
        assert!(!plan.is_blocked());
    }

    #[test]
    fn one_of_several_providers_can_go() {
        let mut first = node("font-a", &[], Reason::Explicit);
        first.provides = names(&["some-font"]);
        let mut second = node("font-b", &[], Reason::Explicit);
        second.provides = names(&["some-font"]);
        let db = PackageDb::new(vec![first, second, node("app", &["some-font"], Reason::Explicit)]);
        let plan = db.plan("font-a", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.remove, names(&["font-a"]));
        assert!(!plan.is_blocked());
    }

    #[test]
    fn the_last_provider_is_held_by_what_needs_it() {
        let mut only = node("font-a", &[], Reason::Explicit);
        only.provides = names(&["some-font"]);
        let db = PackageDb::new(vec![only, node("app", &["some-font"], Reason::Explicit)]);
        let plan = db.plan("font-a", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.blockers, names(&["app"]));
    }

    #[test]
    fn a_critical_package_in_the_list_is_named() {
        let db = PackageDb::new(vec![
            node("app", &["systemd"], Reason::Explicit),
            node("systemd", &[], Reason::Dependency),
        ]);
        let plan = db.plan("app", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.critical, names(&["systemd"]));
    }

    #[test]
    fn freed_space_counts_the_whole_list() {
        let db = PackageDb::new(vec![
            node("app", &["lib"], Reason::Explicit),
            node("lib", &[], Reason::Dependency),
        ]);
        assert_eq!(db.plan("app", Mode::Keep).map(|p| p.freed), Some(20));
    }

    #[test]
    fn a_pulled_in_alternative_stays_while_something_still_uses_it() {
        let mut first = node("backend-a", &[], Reason::Dependency);
        first.provides = names(&["backend"]);
        let mut second = node("backend-b", &[], Reason::Dependency);
        second.provides = names(&["backend"]);
        let db = PackageDb::new(vec![
            first,
            second,
            node("app", &["backend"], Reason::Explicit),
            node("other", &["backend"], Reason::Explicit),
        ]);
        let plan = db.plan("app", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.remove, names(&["app"]));
    }

    #[test]
    fn dependencies_that_only_need_each_other_go_together() {
        let db = PackageDb::new(vec![
            node("app", &["lib"], Reason::Explicit),
            node("lib", &["helper"], Reason::Dependency),
            node("helper", &["lib"], Reason::Dependency),
        ]);
        let plan = db.plan("app", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.remove, names(&["app", "helper", "lib"]));
    }

    #[test]
    fn a_ring_held_from_outside_stays_whole() {
        let db = PackageDb::new(vec![
            node("app", &["lib"], Reason::Explicit),
            node("lib", &["helper"], Reason::Dependency),
            node("helper", &["lib"], Reason::Dependency),
            node("other", &["helper"], Reason::Explicit),
        ]);
        let plan = db.plan("app", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.remove, names(&["app"]));
    }

    #[test]
    fn a_library_version_tells_two_providers_apart() {
        let mut old = node("lib-legacy", &[], Reason::Dependency);
        old.provides = names(&["lib.so=2-64"]);
        let mut new = node("lib", &[], Reason::Dependency);
        new.provides = names(&["lib.so=16-64"]);
        let db = PackageDb::new(vec![
            old,
            new,
            node("old-app", &["lib.so=2-64"], Reason::Explicit),
            node("new-app", &["lib.so=16-64"], Reason::Explicit),
        ]);
        let plan = db.plan("old-app", Mode::Keep).unwrap_or_default();
        assert_eq!(plan.remove, names(&["lib-legacy", "old-app"]));
    }

    #[test]
    fn an_unknown_package_has_no_plan() {
        assert!(PackageDb::default().plan("nothing", Mode::Keep).is_none());
    }

    #[test]
    fn a_version_or_a_description_is_cut_from_a_dependency() {
        assert_eq!(bare_name("foo>=1.2"), "foo");
        assert_eq!(bare_name("foo=1"), "foo");
        assert_eq!(bare_name("foo: for bar"), "foo");
        assert_eq!(bare_name("libfoo.so=1-64"), "libfoo.so");
    }
}
