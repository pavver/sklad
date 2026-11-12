//! The dpkg database of Debian, Ubuntu and their derivatives, read directly.
//!
//! * `/var/lib/dpkg/status` holds one paragraph per package; only installed ones count.
//! * `/var/lib/apt/extended_states` marks the packages apt pulled in as dependencies.
//! * `/var/lib/dpkg/info/<name>.list`, or `<name>:<arch>.list` for co-installable packages,
//!   lists the files of a package; its time of change is when the package was installed.
//! * `apt list --installed` names the suite each package came from, and `local` when none has it.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::package::{Files, Package, classify};
use crate::model::Reason;
use crate::plan::bare_name;

/// One paragraph of a control file: field names to values, continuation lines joined by `\n`.
fn paragraphs(text: &str) -> Vec<HashMap<&str, String>> {
    let mut all = Vec::new();
    let mut current: HashMap<&str, String> = HashMap::new();
    let mut last: Option<&str> = None;
    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                all.push(std::mem::take(&mut current));
            }
            last = None;
        } else if line.starts_with([' ', '\t']) {
            if let Some(value) = last.and_then(|key| current.get_mut(key)) {
                value.push('\n');
                value.push_str(line);
            }
        } else if let Some((key, value)) = line.split_once(':') {
            current.insert(key, value.trim().to_owned());
            last = Some(key);
        }
    }
    if !current.is_empty() {
        all.push(current);
    }
    all
}

/// One relation without its version and architecture: `libc6 (>= 2.34)` -> `libc6`.
fn relation_name(relation: &str) -> &str {
    let end = relation.find(['(', '[', '<']).unwrap_or(relation.len());
    bare_name(relation[..end].trim())
}

/// A `Depends` field as the package graph reads it: one entry per relation, alternatives kept
/// together as `a | b`.
pub fn relations(field: &str) -> Vec<String> {
    field
        .split(',')
        .map(|relation| {
            relation
                .split('|')
                .map(relation_name)
                .filter(|name| !name.is_empty())
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .filter(|relation| !relation.is_empty())
        .collect()
}

/// The summary and the long description of a `Description` field.
fn description(field: &str) -> (String, String) {
    let mut lines = field.lines();
    let summary = lines.next().unwrap_or_default().trim().to_owned();
    let mut long = String::new();
    for line in lines {
        let line = line.strip_prefix(' ').unwrap_or(line);
        if line.trim() == "." {
            long.push_str("\n\n");
        } else {
            if !long.is_empty() && !long.ends_with('\n') {
                long.push(' ');
            }
            long.push_str(line.trim());
        }
    }
    let long = long.trim().to_owned();
    let long = if long.is_empty() { summary.clone() } else { long };
    (summary, long)
}

/// A package as the status file describes it, before its files are read.
#[derive(Clone, Debug)]
pub struct Record {
    pub name: String,
    pub arch: String,
    pub multi_arch_same: bool,
    pub package: Package,
}

pub fn parse_status(text: &str, auto: &HashSet<String>) -> Vec<Record> {
    let mut records: Vec<Record> = paragraphs(text)
        .into_iter()
        .filter(|fields| {
            fields
                .get("Status")
                .is_some_and(|status| status.split_whitespace().nth(2) == Some("installed"))
        })
        .filter_map(|fields| {
            let get = |key: &str| fields.get(key).cloned().unwrap_or_default();
            let name = fields.get("Package")?.clone();
            let arch = get("Architecture");
            let mut depends = relations(&get("Pre-Depends"));
            depends.extend(relations(&get("Depends")));
            let (summary, long) = description(&get("Description"));
            let pulled_in = auto.contains(&name) || auto.contains(&format!("{name}:{arch}"));
            let package = Package {
                name: name.clone(),
                version: get("Version"),
                summary,
                description: long,
                url: fields.get("Homepage").cloned(),
                licenses: Vec::new(),
                groups: fields.get("Section").into_iter().cloned().collect(),
                depends,
                recommends: relations(&get("Recommends")),
                provides: relations(&get("Provides")),
                packager: fields.get("Maintainer").cloned(),
                installed: None,
                size: get("Installed-Size").parse::<u64>().map_or(0, |kib| kib * 1024),
                reason: if pulled_in {
                    Reason::Dependency
                } else {
                    Reason::Explicit
                },
                essential: get("Essential") == "yes" || get("Priority") == "required",
                files: Files::default(),
            };
            Some(Record {
                name,
                arch,
                multi_arch_same: get("Multi-Arch") == "same",
                package,
            })
        })
        .collect();
    // A package installed for two architectures is told apart by its architecture, and still
    // satisfies dependencies by its plain name.
    let mut seen: HashMap<String, usize> = HashMap::new();
    for record in &records {
        *seen.entry(record.name.clone()).or_default() += 1;
    }
    for record in &mut records {
        if seen.get(&record.name).is_some_and(|&count| count > 1) {
            record.package.name = format!("{}:{}", record.name, record.arch);
            record.package.provides.push(record.name.clone());
        }
    }
    records
}

/// Packages apt installed as dependencies, as `name` and `name:arch`.
pub fn parse_extended_states(text: &str) -> HashSet<String> {
    paragraphs(text)
        .into_iter()
        .filter(|fields| fields.get("Auto-Installed").map(String::as_str) == Some("1"))
        .filter_map(|fields| {
            let name = fields.get("Package")?;
            let arch = fields.get("Architecture");
            Some(match arch {
                Some(arch) => format!("{name}:{arch}"),
                None => name.clone(),
            })
        })
        .flat_map(|key| {
            let plain = key.split(':').next().unwrap_or_default().to_owned();
            [key, plain]
        })
        .collect()
}

/// A dpkg file list in the form the classifier reads: relative paths, directories ending in `/`.
/// dpkg writes directories like files, so an entry is a directory when another one lies in it.
/// Merged `/usr` is applied, so `/bin/sh` and `/usr/bin/sh` read the same.
pub fn list_paths(text: &str) -> Vec<String> {
    let mut paths: Vec<String> = text
        .lines()
        .filter_map(|line| line.strip_prefix('/'))
        .filter(|path| !path.is_empty() && *path != ".")
        .map(|path| {
            for (old, new) in [
                ("bin/", "usr/bin/"),
                ("sbin/", "usr/sbin/"),
                ("lib/", "usr/lib/"),
                ("usr/games/", "usr/bin/"),
            ] {
                if let Some(rest) = path.strip_prefix(old) {
                    return format!("{new}{rest}");
                }
                if old.strip_suffix('/') == Some(path) {
                    return new.trim_end_matches('/').to_owned();
                }
            }
            path.to_owned()
        })
        .collect();
    paths.sort();
    paths.dedup();
    let directories: HashSet<String> = paths
        .windows(2)
        .filter(|pair| pair[1].starts_with(&format!("{}/", pair[0])))
        .map(|pair| pair[0].clone())
        .collect();
    paths
        .into_iter()
        .map(|path| {
            if directories.contains(&path) {
                format!("{path}/")
            } else {
                path
            }
        })
        .collect()
}

/// The suite each installed package came from, from `apt list --installed`; `None` for a package
/// no repository has, which apt marks `local`.
pub fn parse_origins(listing: &str) -> HashMap<String, Option<String>> {
    listing
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once('/')?;
            let suites = rest.split_whitespace().next()?;
            let suite = suites.split(',').find(|suite| *suite != "now").map(str::to_owned);
            let local = line.contains("local]") || line.contains(",local");
            Some((name.to_owned(), suite.filter(|_| !local)))
        })
        .collect()
}

/// The time of change of a file, in Unix seconds.
fn changed_at(path: &Path) -> Option<i64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let seconds = modified.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(seconds).ok()
}

pub fn read_all(status: &Path, extended_states: &Path, running: &HashSet<String>) -> Vec<Package> {
    let Ok(text) = fs::read_to_string(status) else {
        return Vec::new();
    };
    let auto = parse_extended_states(&fs::read_to_string(extended_states).unwrap_or_default());
    let info = status.with_file_name("info");
    parse_status(&text, &auto)
        .into_iter()
        .map(|record| {
            let qualified = info.join(format!("{}:{}.list", record.name, record.arch));
            let plain = info.join(format!("{}.list", record.name));
            let list = if record.multi_arch_same && qualified.is_file() {
                qualified
            } else {
                plain
            };
            let paths = list_paths(&fs::read_to_string(&list).unwrap_or_default());
            let mut package = record.package;
            package.files = classify(paths.iter().map(String::as_str), running);
            package.installed = changed_at(&list);
            package
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = "\
Package: gimp
Status: install ok installed
Priority: optional
Section: graphics
Installed-Size: 20480
Maintainer: Debian GNOME Maintainers <pkg-gnome@example.org>
Architecture: amd64
Version: 2.10.34-1
Depends: libc6 (>= 2.34), libgimp2.0 (= 2.10.34-1), python3 | python3-minimal
Recommends: ghostscript
Description: GNU Image Manipulation Program
 GIMP lets you draw, paint, edit images.
 .
 It supports plugins.
Homepage: https://www.gimp.org/

Package: tar
Essential: yes
Status: install ok installed
Priority: required
Architecture: amd64
Version: 1.34

Package: oldtool
Status: deinstall ok config-files
Architecture: amd64

Package: libc6
Status: install ok installed
Multi-Arch: same
Architecture: amd64

Package: libc6
Status: install ok installed
Multi-Arch: same
Architecture: i386
";

    #[test]
    fn installed_packages_are_read_with_their_relations() -> Result<(), String> {
        let auto = parse_extended_states("Package: libc6\nArchitecture: amd64\nAuto-Installed: 1\n");
        let records = parse_status(STATUS, &auto);
        let names: Vec<&str> = records.iter().map(|r| r.package.name.as_str()).collect();
        assert_eq!(names, ["gimp", "tar", "libc6:amd64", "libc6:i386"]);
        let gimp = &records.first().ok_or("no gimp")?.package;
        assert_eq!(gimp.depends, ["libc6", "libgimp2.0", "python3 | python3-minimal"]);
        assert_eq!(gimp.recommends, ["ghostscript"]);
        assert_eq!(gimp.size, 20480 * 1024);
        assert_eq!(gimp.summary, "GNU Image Manipulation Program");
        assert_eq!(
            gimp.description,
            "GIMP lets you draw, paint, edit images.\n\nIt supports plugins."
        );
        assert_eq!(gimp.groups, ["graphics"]);
        assert_eq!(gimp.reason, Reason::Explicit);
        assert!(records.get(1).is_some_and(|tar| tar.package.essential));
        let libc = records.get(2).ok_or("no libc6")?;
        assert_eq!(libc.package.reason, Reason::Dependency);
        assert!(libc.package.provides.contains(&"libc6".to_owned()));
        Ok(())
    }

    #[test]
    fn a_file_list_reads_like_a_pacman_one() {
        let list = "/.\n/usr\n/usr/bin\n/usr/bin/gimp\n/usr/share\n/usr/share/applications\n\
            /usr/share/applications/gimp.desktop\n/bin\n/bin/gimp-console\n/usr/share/doc\n";
        assert_eq!(
            list_paths(list),
            [
                "usr/",
                "usr/bin/",
                "usr/bin/gimp",
                "usr/bin/gimp-console",
                "usr/share/",
                "usr/share/applications/",
                "usr/share/applications/gimp.desktop",
                "usr/share/doc",
            ]
        );
    }

    #[test]
    fn documents_alone_make_a_metapackage() {
        let paths = list_paths(
            "/.\n/usr\n/usr/share\n/usr/share/doc\n/usr/share/doc/x\n/usr/share/doc/x/copyright\n",
        );
        assert!(!classify(paths.iter().map(String::as_str), &HashSet::new()).has_payload);
    }

    #[test]
    fn a_package_no_repository_has_is_local() {
        let origins = parse_origins(
            "Listing...\ngimp/stable,now 2.10 amd64 [installed]\n\
             tool/now 1.0 amd64 [installed,local]\nlibc6/stable-security,stable,now 2.36 amd64 [installed,automatic]\n",
        );
        assert_eq!(origins.get("gimp"), Some(&Some("stable".to_owned())));
        assert_eq!(origins.get("tool"), Some(&None));
        assert_eq!(origins.get("libc6"), Some(&Some("stable-security".to_owned())));
    }
}
