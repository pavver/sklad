//! What a package database says about one package, whichever system keeps it.

use std::collections::HashSet;

use crate::model::{Reason, Section};
use crate::plan::{Node, bare_name};

/// What the file list of a package says about it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Files {
    pub launchers: Vec<String>,
    pub metainfo: Vec<String>,
    pub programs: Vec<String>,
    /// First-level entries under `/opt`, so what lies there unowned can be told apart.
    pub opt_roots: Vec<String>,
    /// Package folders under `/usr/lib/node_modules`.
    pub node_roots: Vec<String>,
    /// `.dist-info` and `.egg-info` records of Python packages.
    pub python_records: Vec<String>,
    pub has_payload: bool,
    pub has_fonts: bool,
    pub has_other_data: bool,
    pub has_kernel_parts: bool,
    pub running: bool,
}

#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub summary: String,
    /// The long description; the summary again when the database has none.
    pub description: String,
    pub url: Option<String>,
    pub licenses: Vec<String>,
    pub groups: Vec<String>,
    pub depends: Vec<String>,
    pub recommends: Vec<String>,
    pub provides: Vec<String>,
    pub packager: Option<String>,
    pub installed: Option<i64>,
    pub size: u64,
    pub reason: Reason,
    /// The package system marks it as needed by every system.
    pub essential: bool,
    pub files: Files,
}

/// The name of the Python metadata record a path lies in, if it does.
fn python_record(path: &str) -> Option<&str> {
    path.split('/')
        .skip_while(|part| *part != "site-packages" && *part != "dist-packages")
        .nth(1)
        .filter(|name| name.ends_with(".dist-info") || name.ends_with(".egg-info"))
}

/// Reads a file list given as paths relative to the root, with directories ending in `/`.
pub fn classify<'a>(paths: impl IntoIterator<Item = &'a str>, running: &HashSet<String>) -> Files {
    let mut facts = Files::default();
    for path in paths {
        facts.note_roots(path);
        if !path.ends_with('/') {
            facts.note_file(path, running);
        }
    }
    facts
}

fn push_new(list: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !list.iter().any(|known| known == value) {
        list.push(value.to_owned());
    }
}

fn is_documentation(path: &str) -> bool {
    path.starts_with("usr/share/licenses/") || path.starts_with("usr/share/doc/")
}

impl Files {
    /// The folders that stand for the package elsewhere: in `/opt`, among the global Node
    /// modules, and among the Python records.
    fn note_roots(&mut self, path: &str) {
        if let Some(rest) = path.strip_prefix("opt/") {
            push_new(&mut self.opt_roots, rest.split('/').next().unwrap_or_default());
        }
        if let Some(rest) = path.strip_prefix("usr/lib/node_modules/") {
            let depth = if rest.starts_with('@') { 2 } else { 1 };
            push_new(
                &mut self.node_roots,
                &rest.split('/').take(depth).collect::<Vec<_>>().join("/"),
            );
        }
        if let Some(record) = python_record(path) {
            push_new(&mut self.python_records, record);
        }
    }

    /// What one file says about the package.
    fn note_file(&mut self, path: &str, running: &HashSet<String>) {
        self.has_payload |= !is_documentation(path);
        self.running |= running.contains(&format!("/{path}"));
        if let Some(name) = path.strip_prefix("usr/share/applications/") {
            if name.ends_with(".desktop") && !name.contains('/') {
                self.launchers.push(path.to_owned());
            }
        } else if path.starts_with("usr/share/metainfo/") {
            if path.ends_with(".xml") {
                self.metainfo.push(path.to_owned());
            }
        } else if let Some(name) = path.strip_prefix("usr/bin/") {
            self.programs.push(name.to_owned());
        } else {
            self.note_data(path);
        }
    }

    /// A file that is neither a launcher, nor metadata, nor a program.
    fn note_data(&mut self, path: &str) {
        if path.starts_with("usr/share/fonts/") {
            self.has_fonts = true;
        } else if path.starts_with("usr/lib/modules/") || path.starts_with("usr/lib/firmware/") {
            self.has_kernel_parts = true;
        } else if !is_documentation(path) {
            self.has_other_data = true;
        }
    }
}

impl Package {
    pub fn is_meta(&self) -> bool {
        !self.files.has_payload
    }

    pub fn node(&self) -> Node {
        Node {
            name: self.name.clone(),
            depends: self.depends.clone(),
            recommends: self.recommends.clone(),
            provides: self.provides.clone(),
            reason: self.reason,
            critical: self.essential,
            meta: self.is_meta(),
            size: self.size,
        }
    }

    /// Dependencies by name, for reading: versions mean nothing to the person.
    pub fn depends_by_name(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .depends
            .iter()
            .flat_map(|dep| dep.split('|'))
            .map(|dep| bare_name(dep).to_owned())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// The section of a package with no visible launcher.
    pub fn section_without_window(&self) -> Section {
        const DRIVER_PREFIXES: [&str; 6] = ["linux", "nvidia", "mesa", "vulkan-", "xf86-", "lib32-nvidia"];
        const DRIVER_SUFFIXES: [&str; 3] = ["-firmware", "-dkms", "-ucode"];
        let name = self.name.as_str();
        let driver = self.files.has_kernel_parts
            || DRIVER_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
            || DRIVER_SUFFIXES.iter().any(|suffix| name.ends_with(suffix));
        if driver {
            Section::Drivers
        } else if self.is_meta() {
            Section::Meta
        } else if self.files.has_fonts && self.files.programs.is_empty() {
            Section::Fonts
        } else if self.files.programs.is_empty() {
            Section::Libraries
        } else {
            Section::Console
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The section of a package with no launcher, named so its name says nothing, that holds
    /// `paths`.
    fn section_of(paths: &[&str]) -> Section {
        Package {
            name: "plain".to_owned(),
            version: String::new(),
            summary: String::new(),
            description: String::new(),
            url: None,
            licenses: Vec::new(),
            groups: Vec::new(),
            depends: Vec::new(),
            recommends: Vec::new(),
            provides: Vec::new(),
            packager: None,
            installed: None,
            size: 0,
            reason: Reason::Explicit,
            essential: false,
            files: classify(paths.iter().copied(), &HashSet::new()),
        }
        .section_without_window()
    }

    #[test]
    fn the_files_of_a_package_tell_its_section() {
        assert_eq!(section_of(&["usr/share/fonts/TTF/plain.ttf"]), Section::Fonts);
        assert_eq!(
            section_of(&["usr/lib/modules/6.1/extra/plain.ko"]),
            Section::Drivers
        );
        assert_eq!(section_of(&["usr/lib/firmware/plain.bin"]), Section::Drivers);
        assert_eq!(section_of(&["usr/lib/libplain.so"]), Section::Libraries);
        assert_eq!(section_of(&["usr/share/doc/plain/README"]), Section::Meta);
        assert_eq!(section_of(&["usr/bin/plain"]), Section::Console);
    }
}
