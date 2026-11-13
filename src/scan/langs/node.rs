//! Node.js packages installed by npm and pnpm.

use std::fs;
use std::path::{Path, PathBuf};

use super::{Owned, Scan, manager, tool, words};
use crate::model::{Access, Item, Removal, Section, Source};
use crate::scan::{Places, blank, loose};

/// What `package.json` says about a package.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodePackage {
    pub name: String,
    pub version: String,
    pub summary: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub author: Option<String>,
    pub has_programs: bool,
}

/// A field that is either a plain string or an object holding it under a key.
pub(super) fn text_or_field(value: Option<&serde_json::Value>, key: &str) -> Option<String> {
    let value = value?;
    value
        .as_str()
        .or_else(|| value.get(key).and_then(|inner| inner.as_str()))
        .map(str::to_owned)
}

pub fn parse_package_json(text: &str) -> Option<NodePackage> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let field = |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    Some(NodePackage {
        name: field("name")?,
        version: field("version").unwrap_or_default(),
        summary: field("description").unwrap_or_default(),
        license: text_or_field(value.get("license"), "type"),
        homepage: field("homepage"),
        author: text_or_field(value.get("author"), "name"),
        has_programs: value.get("bin").is_some(),
    })
}

/// Package folders of a `node_modules`, with scoped ones one level deeper.
pub fn node_packages_in(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for folder in loose::directories_in(root) {
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.starts_with('@') {
            found.extend(loose::directories_in(&folder));
        } else if !name.starts_with('.') {
            found.push(folder);
        }
    }
    found
}

pub(super) fn node_item(folder: &Path, package: NodePackage, source_name: &str, removal: Removal) -> Item {
    let id = format!("{source_name}:{}", folder.display());
    let mut item = blank(id, package.name.clone(), Source::Node, removal);
    item.version = package.version;
    item.summary = package.summary.clone();
    item.description = package.summary;
    item.licenses = package.license.into_iter().collect();
    item.homepage = package
        .homepage
        .or(Some(format!("https://www.npmjs.com/package/{}", package.name)));
    item.developer = package.author;
    item.size = Some(loose::size_of(folder));
    item.section = if package.has_programs {
        Section::Console
    } else {
        Section::Libraries
    };
    item
}

pub(super) fn read_node(folder: &Path) -> Option<NodePackage> {
    parse_package_json(&fs::read_to_string(folder.join("package.json")).ok()?)
}

pub(super) fn npm_items(places: &Places, owned: &Owned, out: &mut Scan) {
    let user_prefix = places.home.join(".local");
    let user_root = user_prefix.join("lib/node_modules");
    for folder in node_packages_in(&user_root) {
        let Some(package) = read_node(&folder) else {
            continue;
        };
        let prefix = user_prefix.display().to_string();
        let name = package.name.clone();
        let argv = words(&["npm", "uninstall", "-g", "--prefix", &prefix, &name]);
        out.items.push(node_item(
            &folder,
            package,
            "npm",
            manager("npm", argv, Access::User),
        ));
    }
    out.claimed.push(user_root);

    let system_root = places.root.join("usr/lib/node_modules");
    for folder in node_packages_in(&system_root) {
        let relative = folder
            .strip_prefix(&system_root)
            .map(Path::to_path_buf)
            .unwrap_or_default();
        if owned.node.contains(&relative.display().to_string()) {
            continue;
        }
        let Some(package) = read_node(&folder) else {
            continue;
        };
        let name = package.name.clone();
        let argv = words(&["npm", "uninstall", "-g", &name]);
        out.items.push(node_item(
            &folder,
            package,
            "npm",
            manager("npm", argv, Access::Root),
        ));
    }
}

pub(super) fn pnpm_items(places: &Places, out: &mut Scan) {
    let global = places.home.join(".local/share/pnpm/global");
    for version in loose::directories_in(&global) {
        for folder in node_packages_in(&version.join("node_modules")) {
            let Some(package) = read_node(&folder) else {
                continue;
            };
            let argv = vec![
                tool(places, "pnpm"),
                "remove".to_owned(),
                "-g".to_owned(),
                package.name.clone(),
            ];
            out.items.push(node_item(
                &folder,
                package,
                "pnpm",
                manager("pnpm", argv, Access::User),
            ));
        }
    }
    out.claimed.push(global);
}
