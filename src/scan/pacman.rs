//! The local pacman database, read directly: one `pacman -Qi` per package would take minutes.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::package::{Files, Package, classify};
use crate::model::Reason;
use crate::plan::exact_spec;

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
        description: one("DESC").unwrap_or_default().to_owned(),
        url: one("URL").map(str::to_owned),
        licenses: many("LICENSE"),
        groups: many("GROUPS"),
        depends: specs("DEPENDS"),
        recommends: Vec::new(),
        provides: specs("PROVIDES"),
        packager: one("PACKAGER").map(str::to_owned),
        installed: one("INSTALLDATE").and_then(|value| value.parse().ok()),
        size: one("SIZE").and_then(|value| value.parse().ok()).unwrap_or(0),
        reason: match one("REASON") {
            Some("1") => Reason::Dependency,
            _ => Reason::Explicit,
        },
        essential: false,
        files,
    })
}

pub fn parse_files(text: &str, running: &HashSet<String>) -> Files {
    let list = blocks(text);
    classify(list.get("FILES").into_iter().flatten().copied(), running)
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
