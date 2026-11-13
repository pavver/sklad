//! Items of the package managers sklad reads whole: the distribution, Flatpak, Steam and Cargo.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::i18n::Lang;
use crate::model::{Evidence, Item, Launch, PackageSystem, Removal, Section, Source, Usage};

use super::appstream::Metainfo;
use super::desktop::Launcher;
use super::package::Package;
use super::{
    Context, appstream, blank, capture, desktop, dress, flatpak, loose, read_launchers, read_metainfo, steam,
    usage_of,
};

/// Where a package of the distribution came from: the repository that has it, or else the AUR
/// on pacman and a local file on dpkg.
fn package_source(name: &str, system: PackageSystem, repositories: &HashMap<String, String>) -> Source {
    let plain = name.split(':').next().unwrap_or(name);
    match (repositories.get(name).or_else(|| repositories.get(plain)), system) {
        (Some(repository), _) => Source::Repo(repository.clone()),
        (None, PackageSystem::Pacman) => Source::Aur,
        (None, PackageSystem::Dpkg) => Source::LocalDeb,
    }
}

pub(super) fn package_item(
    package: &Package,
    system: PackageSystem,
    repositories: &HashMap<String, String>,
    extra_launchers: &[Launcher],
    context: &Context<'_>,
) -> Item {
    let prefix = match system {
        PackageSystem::Pacman => "pacman",
        PackageSystem::Dpkg => "dpkg",
    };
    let mut item = blank(
        format!("{prefix}:{}", package.name),
        package.name.clone(),
        package_source(&package.name, system, repositories),
        Removal::Package(system),
    );
    item.version = package.version.clone();
    item.summary = package.summary.clone();
    item.description = package.description.clone();
    item.licenses = package.licenses.clone();
    item.homepage = package.url.clone();
    item.developer = package.packager.clone().filter(|name| name != "Unknown Packager");
    item.reason = Some(package.reason);
    item.size = Some(package.size);
    item.installed = package.installed;
    item.groups = package.groups.clone();
    item.depends = package.depends_by_name();
    item.section = package.section_without_window();

    let mut launchers = read_launchers(&context.places.root, &package.files.launchers, context.lang);
    launchers.extend_from_slice(extra_launchers);
    let info = read_metainfo(&context.places.root, &package.files.metainfo, context.lang);
    dress(&mut item, &launchers, &info, context.icons);
    if launchers.iter().any(|launcher| launcher.shown) {
        item.usage = usage_of(&launchers, context.records);
    }
    if package.files.running && item.section.is_app() {
        item.usage = Usage::Running;
    }
    item
}

pub(super) fn flatpak_items(context: &Context<'_>) -> Vec<Item> {
    let listing = capture("flatpak", &["list", &format!("--columns={}", flatpak::COLUMNS)]);
    let home = &context.places.home;
    let roots = [
        context.places.system_path("var/lib/flatpak"),
        home.join(".local/share/flatpak"),
    ];
    flatpak::parse(&listing)
        .into_iter()
        .map(|entry| {
            let removal = Removal::Flatpak {
                app_id: entry.app_id.clone(),
                scope: entry.scope,
            };
            let mut item = blank(
                format!("flatpak:{}", entry.app_id),
                entry.app_id.clone(),
                Source::Flatpak,
                removal,
            );
            item.title = entry.name.clone();
            item.version = entry.version.clone();
            item.summary = entry.summary.clone();
            item.description = entry.summary.clone();
            item.size = entry.size;
            item.section = Section::Libraries;
            let launchers: Vec<Launcher> = roots
                .iter()
                .map(|root| root.join(format!("exports/share/applications/{}.desktop", entry.app_id)))
                .filter_map(|path| desktop::read(&path, context.lang))
                .collect();
            let info = roots
                .iter()
                .find_map(|root| flatpak_metainfo(root, &entry.app_id, context.lang));
            dress(&mut item, &launchers, &info.unwrap_or_default(), context.icons);
            item.title = if launchers.is_empty() {
                entry.name
            } else {
                item.title
            };
            if !entry.runtime && !launchers.is_empty() {
                item.usage = usage_of(&launchers, context.records);
            }
            item
        })
        .collect()
}

fn flatpak_metainfo(root: &Path, app_id: &str, lang: Lang) -> Option<Metainfo> {
    let share = root.join(format!("app/{app_id}/current/active/files/share"));
    ["metainfo", "appdata"].iter().find_map(|folder| {
        ["metainfo.xml", "appdata.xml"].iter().find_map(|suffix| {
            appstream::read(&share.join(format!("{folder}/{app_id}.{suffix}")), lang.code())
        })
    })
}

pub(super) fn steam_items(context: &Context<'_>) -> Vec<Item> {
    let root = context.places.home.join(".local/share/Steam");
    steam::read_all(&root)
        .into_iter()
        .map(|game| {
            let removal = Removal::Steam {
                app_id: game.app_id.clone(),
            };
            let mut item = blank(
                format!("steam:{}", game.app_id),
                game.app_id.clone(),
                Source::Steam,
                removal,
            );
            item.title = game.name.clone();
            item.size = game.size;
            item.homepage = Some(format!("https://store.steampowered.com/app/{}", game.app_id));
            item.icon = context.icons.find(&format!("steam_icon_{}", game.app_id));
            if steam::is_tool(&game.name) {
                item.section = Section::Libraries;
                item.summary = context.lang.text().steam_component.to_owned();
            } else {
                item.section = Section::Games;
                item.summary = context.lang.text().steam_game.to_owned();
                item.launch = Some(Launch::Steam {
                    app_id: game.app_id.clone(),
                });
                item.usage = game.last_played.map_or(Usage::NoRecord, |at| Usage::Last {
                    at,
                    by: Evidence::Steam,
                });
            }
            item.description = item.summary.clone();
            item
        })
        .collect()
}

pub(super) fn cargo_items(context: &Context<'_>) -> (Vec<Item>, HashSet<PathBuf>) {
    let bin = context.places.home.join(".cargo/bin");
    let file = context.places.home.join(".cargo/.crates2.json");
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let mut claimed = HashSet::new();
    let items = loose::parse_crates(&text)
        .into_iter()
        .map(|krate| {
            let removal = Removal::Cargo {
                name: krate.name.clone(),
            };
            let mut item = blank(
                format!("cargo:{}", krate.name),
                krate.name.clone(),
                Source::Cargo,
                removal,
            );
            item.version = krate.version.clone();
            item.summary = (context.lang.text().programs)(&krate.binaries.join(", "));
            item.description = item.summary.clone();
            item.homepage = Some(format!("https://crates.io/crates/{}", krate.name));
            let paths: Vec<PathBuf> = krate.binaries.iter().map(|name| bin.join(name)).collect();
            item.size = Some(paths.iter().map(|path| loose::size_of(path)).sum());
            claimed.extend(paths);
            item
        })
        .collect();
    (items, claimed)
}
