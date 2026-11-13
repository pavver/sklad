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
    /// Each entry names one package, or alternatives as `a | b`, any of which will do.
    pub depends: Vec<String>,
    /// Weak dependencies: they do not break anything when gone, but they keep a package that was
    /// pulled in, the way apt keeps recommended packages from autoremoval.
    pub recommends: Vec<String>,
    pub provides: Vec<String>,
    pub reason: Reason,
    /// The package system itself says the system needs it, as Debian does with `Essential: yes`.
    pub critical: bool,
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
    wanted_by: Vec<Vec<usize>>,
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
pub struct Plan {
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

impl Plan {
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

fn push_once(list: &mut Vec<usize>, index: usize) {
    if !list.contains(&index) {
        list.push(index);
    }
}

type Providers = HashMap<String, Vec<usize>>;

/// Every name a package answers to: its own and what it provides, bare and versioned.
fn providers(nodes: &[Node]) -> Providers {
    let mut providers: Providers = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        providers.entry(node.name.clone()).or_default().push(index);
        for provided in &node.provides {
            for key in [provided.as_str(), bare_name(provided)] {
                push_once(providers.entry(key.to_owned()).or_default(), index);
            }
        }
    }
    providers
}

/// Alternatives are one dependency: it holds while any of them is installed.
fn add_alternatives(nodes: &[Node], providers: &mut Providers) {
    let alternatives: Vec<String> = nodes
        .iter()
        .flat_map(|node| node.depends.iter().chain(&node.recommends))
        .filter(|dep| dep.contains('|'))
        .cloned()
        .collect();
    for dep in alternatives {
        let mut any = Vec::new();
        for choice in dep.split('|').map(str::trim) {
            let found = providers.get(choice).or_else(|| providers.get(bare_name(choice)));
            for &provider in found.map_or(&[][..], Vec::as_slice) {
                push_once(&mut any, provider);
            }
        }
        providers.insert(dep, any);
    }
}

/// For each package, the others that name it in `list`.
fn users(nodes: &[Node], providers: &Providers, list: fn(&Node) -> &Vec<String>) -> Vec<Vec<usize>> {
    let mut users = vec![Vec::new(); nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        for dep in list(node) {
            let exact = providers.get(dep).or_else(|| providers.get(bare_name(dep)));
            for &provider in exact.map_or(&[][..], Vec::as_slice) {
                if provider != index {
                    push_once(&mut users[provider], index);
                }
            }
        }
    }
    users
}

impl PackageDb {
    pub fn new(nodes: Vec<Node>) -> Self {
        let by_name = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.name.clone(), index))
            .collect();
        let mut providers = providers(&nodes);
        add_alternatives(&nodes, &mut providers);
        let needed_by = users(&nodes, &providers, |node| &node.depends);
        let wanted_by = users(&nodes, &providers, |node| &node.recommends);
        Self {
            nodes,
            by_name,
            providers,
            needed_by,
            wanted_by,
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

    pub fn plan(&self, target: &str, mode: Mode) -> Option<Plan> {
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
        let critical = gone
            .iter()
            .map(|&i| &self.nodes[i])
            .filter(|node| node.critical || is_critical(&node.name))
            .map(|node| node.name.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut keep_names: Vec<String> = keep.iter().map(|&i| self.nodes[i].name.clone()).collect();
        keep_names.sort();
        Some(Plan {
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
    /// What the removed packages pulled in, followed down: each may become unneeded.
    fn pulled_in_by(&self, gone: &HashSet<usize>, keep: &BTreeSet<usize>) -> HashSet<usize> {
        let mut candidates = HashSet::new();
        let mut frontier: Vec<usize> = gone.iter().copied().collect();
        while let Some(current) = frontier.pop() {
            let node = &self.nodes[current];
            for dep in node.depends.iter().chain(&node.recommends) {
                for &provider in self.providers_of(dep) {
                    let found = &self.nodes[provider];
                    // apt never autoremoves an essential or required package.
                    let removable = found.reason == Reason::Dependency && !found.critical;
                    if removable
                        && !gone.contains(&provider)
                        && !keep.contains(&provider)
                        && candidates.insert(provider)
                    {
                        frontier.push(provider);
                    }
                }
            }
        }
        candidates
    }

    /// Whether a package that stays needs or wants this one.
    fn is_held(&self, candidate: usize, gone: &HashSet<usize>, candidates: &HashSet<usize>) -> bool {
        self.needed_by[candidate]
            .iter()
            .chain(&self.wanted_by[candidate])
            .any(|user| !gone.contains(user) && !candidates.contains(user))
    }

    fn add_unneeded(&self, gone: &mut HashSet<usize>, keep: &BTreeSet<usize>) {
        let mut candidates = self.pulled_in_by(gone, keep);
        loop {
            let held: Vec<usize> = candidates
                .iter()
                .copied()
                .filter(|candidate| self.is_held(*candidate, gone, &candidates))
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
            recommends: Vec::new(),
            provides: Vec::new(),
            reason,
            critical: false,
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

    #[test]
    fn a_dependency_on_alternatives_holds_while_one_of_them_stays() {
        let db = PackageDb::new(vec![
            node("app", &["mawk | gawk"], Reason::Explicit),
            node("mawk", &[], Reason::Explicit),
            node("gawk", &[], Reason::Explicit),
        ]);
        assert_eq!(
            db.plan("mawk", Mode::Keep).map(|p| p.remove),
            Some(names(&["mawk"]))
        );
        assert_eq!(db.required_by("gawk"), names(&["app"]));
    }

    #[test]
    fn a_recommended_package_stays_while_what_recommends_it_stays() {
        let mut viewer = node("viewer", &[], Reason::Explicit);
        viewer.recommends = names(&["codecs"]);
        let db = PackageDb::new(vec![
            node("player", &["codecs"], Reason::Explicit),
            viewer,
            node("codecs", &[], Reason::Dependency),
        ]);
        assert_eq!(
            db.plan("player", Mode::Keep).map(|p| p.remove),
            Some(names(&["player"]))
        );
        assert_eq!(
            db.plan("viewer", Mode::Keep).map(|p| p.remove),
            Some(names(&["viewer"]))
        );
    }

    #[test]
    fn what_a_removed_package_recommended_goes_with_it() {
        let mut viewer = node("viewer", &[], Reason::Explicit);
        viewer.recommends = names(&["codecs"]);
        let db = PackageDb::new(vec![viewer, node("codecs", &[], Reason::Dependency)]);
        assert_eq!(
            db.plan("viewer", Mode::Keep).map(|p| p.remove),
            Some(names(&["codecs", "viewer"]))
        );
    }

    #[test]
    fn an_essential_package_is_critical_whatever_its_name() {
        let mut tar = node("tar", &[], Reason::Explicit);
        tar.critical = true;
        let db = PackageDb::new(vec![tar]);
        assert_eq!(
            db.plan("tar", Mode::Keep).map(|p| p.critical),
            Some(names(&["tar"]))
        );
    }

    #[test]
    fn an_essential_dependency_stays_when_its_user_goes() {
        let mut tzdata = node("tzdata", &[], Reason::Dependency);
        tzdata.critical = true;
        let db = PackageDb::new(vec![node("python", &["tzdata"], Reason::Explicit), tzdata]);
        assert_eq!(
            db.plan("python", Mode::Keep).map(|p| p.remove),
            Some(names(&["python"]))
        );
    }
}
