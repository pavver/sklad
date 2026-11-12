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
        if let Some(rest) = path.strip_prefix("opt/") {
            let root = rest.split('/').next().unwrap_or_default();
            if !root.is_empty() && !facts.opt_roots.iter().any(|known| known == root) {
                facts.opt_roots.push(root.to_owned());
            }
        }
        if let Some(rest) = path.strip_prefix("usr/lib/node_modules/") {
            let depth = if rest.starts_with('@') { 2 } else { 1 };
            let root = rest.split('/').take(depth).collect::<Vec<_>>().join("/");
            if !root.is_empty() && !facts.node_roots.contains(&root) {
                facts.node_roots.push(root);
            }
        }
        if let Some(record) = python_record(path)
            && !facts.python_records.iter().any(|known| known == record)
        {
            facts.python_records.push(record.to_owned());
        }
        if path.ends_with('/') {
            continue;
        }
        if !path.starts_with("usr/share/licenses/") && !path.starts_with("usr/share/doc/") {
            facts.has_payload = true;
        }
        if running.contains(&format!("/{path}")) {
            facts.running = true;
        }
        if let Some(name) = path.strip_prefix("usr/share/applications/") {
            if name.ends_with(".desktop") && !name.contains('/') {
                facts.launchers.push(path.to_owned());
            }
        } else if path.starts_with("usr/share/metainfo/") {
            if path.ends_with(".xml") {
                facts.metainfo.push(path.to_owned());
            }
        } else if let Some(name) = path.strip_prefix("usr/bin/") {
            facts.programs.push(name.to_owned());
        } else if path.starts_with("usr/share/fonts/") {
            facts.has_fonts = true;
        } else if path.starts_with("usr/lib/modules/") || path.starts_with("usr/lib/firmware/") {
            facts.has_kernel_parts = true;
        } else if !path.starts_with("usr/share/licenses/") && !path.starts_with("usr/share/doc/") {
            facts.has_other_data = true;
        }
    }
    facts
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
