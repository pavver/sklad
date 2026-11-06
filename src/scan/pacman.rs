//! The local pacman database, read directly: one `pacman -Qi` per package would take minutes.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::model::{Reason, Section};
use crate::plan::{Node, bare_name, exact_spec};

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
    pub url: Option<String>,
    pub licenses: Vec<String>,
    pub groups: Vec<String>,
    pub depends: Vec<String>,
    pub provides: Vec<String>,
    pub packager: Option<String>,
    pub installed: Option<i64>,
    pub size: u64,
    pub reason: Reason,
    pub files: Files,
}

/// Splits a pacman record into its `%KEY%` blocks.
fn blocks(text: &str) -> HashMap<&str, Vec<&str>> {
    let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut key = None;
    for line in text.lines() {
        if line.len() > 2 && line.starts_with('%') && line.ends_with('%') {
            key = Some(&line[1..line.len() - 1]);
            continue;
        }
        if let Some(key) = key.filter(|_| !line.is_empty()) {
            map.entry(key).or_default().push(line);
        }
    }
    map
}

pub fn parse_desc(desc: &str, files: Files) -> Option<Package> {
    let map = blocks(desc);
    let one = |key: &str| map.get(key).and_then(|lines| lines.first()).copied();
    let many = |key: &str| -> Vec<String> {
        map.get(key)
            .map(|lines| lines.iter().map(|line| (*line).to_owned()).collect())
            .unwrap_or_default()
    };
    let specs =
        |key: &str| -> Vec<String> { many(key).iter().map(|line| exact_spec(line).to_owned()).collect() };
    Some(Package {
        name: one("NAME")?.to_owned(),
        version: one("VERSION").unwrap_or_default().to_owned(),
        summary: one("DESC").unwrap_or_default().to_owned(),
        url: one("URL").map(str::to_owned),
        licenses: many("LICENSE"),
        groups: many("GROUPS"),
        depends: specs("DEPENDS"),
        provides: specs("PROVIDES"),
        packager: one("PACKAGER").map(str::to_owned),
        installed: one("INSTALLDATE").and_then(|value| value.parse().ok()),
        size: one("SIZE").and_then(|value| value.parse().ok()).unwrap_or(0),
        reason: match one("REASON") {
            Some("1") => Reason::Dependency,
            _ => Reason::Explicit,
        },
        files,
    })
}

/// The name of the Python metadata record a path lies in, if it does.
fn python_record(path: &str) -> Option<&str> {
    path.split('/')
        .skip_while(|part| *part != "site-packages")
        .nth(1)
        .filter(|name| name.ends_with(".dist-info") || name.ends_with(".egg-info"))
}

pub fn parse_files(text: &str, running: &HashSet<String>) -> Files {
    let mut facts = Files::default();
    let list = blocks(text);
    for &path in list.get("FILES").map_or(&[][..], Vec::as_slice) {
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

pub fn read_all(dir: &Path, running: &HashSet<String>) -> Vec<Package> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let desc = fs::read_to_string(path.join("desc")).ok()?;
            let list = fs::read_to_string(path.join("files")).unwrap_or_default();
            parse_desc(&desc, parse_files(&list, running))
        })
        .collect()
}

impl Package {
    pub fn is_meta(&self) -> bool {
        !self.files.has_payload
    }

    pub fn node(&self) -> Node {
        Node {
            name: self.name.clone(),
            depends: self.depends.clone(),
            provides: self.provides.clone(),
            reason: self.reason,
            meta: self.is_meta(),
            size: self.size,
        }
    }

    /// Dependencies by name, for reading: versions mean nothing to the person.
    pub fn depends_by_name(&self) -> Vec<String> {
        let mut names: Vec<String> = self.depends.iter().map(|dep| bare_name(dep).to_owned()).collect();
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

/// Repository of every package the sync databases know, from `pacman -Sl`.
pub fn parse_repositories(listing: &str) -> HashMap<String, String> {
    listing
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let repo = words.next()?;
            let name = words.next()?;
            Some((name.to_owned(), repo.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESC: &str = "%NAME%\nbrave-bin\n\n%VERSION%\n1:1.98-1\n\n%DESC%\nWeb browser\n\n\
        %URL%\nhttps://brave.com\n\n%INSTALLDATE%\n1793809265\n\n%SIZE%\n500\n\n%REASON%\n1\n\n\
        %LICENSE%\nMPL2\nBSD\n\n%DEPENDS%\ngtk3>=3.0\nnss\n\n%PROVIDES%\nbrave=1.98\n\n\
        %OPTDEPENDS%\ncups: printing\n\n";

    #[test]
    fn a_record_is_read_and_constraints_are_cut() -> Result<(), String> {
        let package = parse_desc(DESC, Files::default()).ok_or("no package")?;
        assert_eq!(package.name, "brave-bin");
        assert_eq!(package.licenses, ["MPL2", "BSD"]);
        assert_eq!(package.depends, ["gtk3", "nss"]);
        assert_eq!(package.provides, ["brave=1.98"]);
        assert_eq!(package.reason, Reason::Dependency);
        assert_eq!(package.installed, Some(1793809265));
        Ok(())
    }

    #[test]
    fn a_record_without_a_reason_was_installed_on_purpose() -> Result<(), String> {
        let package = parse_desc("%NAME%\nx\n", Files::default()).ok_or("no package")?;
        assert_eq!(package.reason, Reason::Explicit);
        Ok(())
    }

    #[test]
    fn licences_and_documents_alone_make_a_metapackage() {
        let list = "%FILES%\nusr/\nusr/share/\nusr/share/licenses/x/LICENSE\nusr/share/doc/x/a\n";
        assert!(!parse_files(list, &HashSet::new()).has_payload);
    }

    #[test]
    fn the_file_list_says_what_the_package_is() {
        let list = "%FILES%\nusr/bin/app\nusr/share/applications/app.desktop\n\
            usr/share/applications/sub/x.desktop\nusr/share/metainfo/app.metainfo.xml\n\
            opt/app/\nopt/app/run\n";
        let running = HashSet::from(["/usr/bin/app".to_owned()]);
        let facts = parse_files(list, &running);
        assert_eq!(facts.programs, ["app"]);
        assert_eq!(facts.launchers, ["usr/share/applications/app.desktop"]);
        assert_eq!(facts.opt_roots, ["app"]);
        let list = "%FILES%\nusr/lib/node_modules/npm/index.js\nusr/lib/node_modules/@a/b/x.js\n\
            usr/lib/python3.14/site-packages/pip-26.dist-info/\n\
            usr/lib/python3.14/site-packages/pip-26.dist-info/METADATA\n";
        let facts_of_tools = parse_files(list, &HashSet::new());
        assert_eq!(facts_of_tools.node_roots, ["npm", "@a/b"]);
        assert_eq!(facts_of_tools.python_records, ["pip-26.dist-info"]);
        assert!(facts.running && facts.has_payload);
    }

    #[test]
    fn a_translated_marker_does_not_disturb_the_repository_list() {
        let map = parse_repositories("core acl 2.4.0-1 [встановлено]\nextra zip 3.0-1\n");
        assert_eq!(map.get("acl").map(String::as_str), Some("core"));
        assert_eq!(map.get("zip").map(String::as_str), Some("extra"));
    }
}
