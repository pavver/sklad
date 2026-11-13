//! Builds the catalogue from every source. Runs on a background thread.

pub mod appstream;
pub mod desktop;
pub mod docker;
pub mod dpkg;
pub mod flatpak;
pub mod langs;
pub mod loose;
mod manual;
pub mod package;
pub mod pacman;
mod sources;
pub mod steam;
pub mod system;
pub mod usage;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::i18n::Lang;
use crate::icons::IconIndex;
use crate::model::{Evidence, Item, Launch, PackageSystem, Removal, Section, Source, Usage};
use crate::plan::PackageDb;
use appstream::Metainfo;
use desktop::Launcher;
use manual::manual_items;
use package::Package;
use sources::{cargo_items, flatpak_items, package_item, steam_items};

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub items: Vec<Item>,
    pub db: PackageDb,
    /// The package system of the distribution, `None` when there is none the catalogue reads.
    pub system: Option<PackageSystem>,
    /// The icons of sections and sources found in the icon themes, by name.
    pub theme_icons: HashMap<&'static str, PathBuf>,
}

/// Looks up every name the side bar may ask for.
fn theme_icons(icons: &IconIndex) -> HashMap<&'static str, PathBuf> {
    Section::APPS
        .iter()
        .chain(&Section::OTHER)
        .flat_map(|section| section.theme_icons())
        .filter_map(|&name| icons.find(name).map(|path| (name, path)))
        .collect()
}

/// The places the scan reads, so a test can point it elsewhere.
#[derive(Clone, Debug)]
pub struct Places {
    pub home: PathBuf,
    pub pacman_local: PathBuf,
    pub dpkg_status: PathBuf,
    pub apt_extended_states: PathBuf,
    pub os_release: PathBuf,
    pub root: PathBuf,
}

impl Places {
    pub fn system() -> Self {
        Self {
            home: std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default(),
            pacman_local: PathBuf::from("/var/lib/pacman/local"),
            dpkg_status: PathBuf::from("/var/lib/dpkg/status"),
            apt_extended_states: PathBuf::from("/var/lib/apt/extended_states"),
            os_release: PathBuf::from("/etc/os-release"),
            root: PathBuf::from("/"),
        }
    }

    fn system_path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }
}

/// Runs a tool in the C locale: its output is parsed, and a translated one would not match.
fn capture(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

/// A blank item of a source; each scanner fills in what it knows.
fn blank(id: String, package: String, source: Source, removal: Removal) -> Item {
    Item {
        id,
        title: package.clone(),
        package,
        version: String::new(),
        summary: String::new(),
        description: String::new(),
        licenses: Vec::new(),
        homepage: None,
        developer: None,
        source,
        section: Section::Console,
        reason: None,
        size: None,
        installed: None,
        usage: Usage::NotTracked,
        icon: None,
        launchers: Vec::new(),
        launch: None,
        docker: None,
        groups: Vec::new(),
        depends: Vec::new(),
        required_by: Vec::new(),
        removal,
        warning: None,
        haystack: String::new(),
    }
}

/// The last launch of whichever of the launchers was started most recently.
fn usage_of(launchers: &[Launcher], records: &HashMap<String, i64>) -> Usage {
    launchers
        .iter()
        .filter_map(|launcher| records.get(&usage::key(&launcher.id)))
        .max()
        .map_or(Usage::NoRecord, |&at| Usage::Last {
            at,
            by: Evidence::Plasma,
        })
}

/// The launcher that stands for the package: the one named after it, or the first one shown.
fn main_launcher<'a>(name: &str, launchers: &'a [Launcher]) -> Option<&'a Launcher> {
    let shown = || launchers.iter().filter(|launcher| launcher.shown);
    let lower = name.to_lowercase();
    shown()
        .find(|launcher| launcher.id.to_lowercase().ends_with(&lower))
        .or_else(|| shown().next())
}

fn read_launchers(root: &Path, files: &[String], lang: Lang) -> Vec<Launcher> {
    files
        .iter()
        .filter_map(|file| desktop::read(&root.join(file), lang))
        .collect()
}

/// Applies a launcher and AppStream data over what the package database says.
fn dress(item: &mut Item, launchers: &[Launcher], info: &Metainfo, icons: &IconIndex) {
    let main = main_launcher(&item.package, launchers);
    if let Some(main) = main {
        item.title = main.name.clone();
        item.section = desktop::section_of(&main.categories);
        item.icon = main.icon.as_deref().and_then(|name| icons.find(name));
        if let Some(comment) = &main.comment {
            item.summary = comment.clone();
        }
        if !main.path.as_os_str().is_empty() {
            item.launch = Some(Launch::Desktop(main.path.clone()));
        }
    }
    item.launchers = launchers
        .iter()
        .filter(|launcher| launcher.shown)
        .map(|launcher| launcher.name.clone())
        .collect();
    if main.is_none()
        && let Some(summary) = &info.summary
    {
        item.summary = summary.clone();
    }
    if let Some(description) = &info.description {
        item.description = description.clone();
    }
    if let Some(license) = &info.license {
        item.licenses = vec![license.clone()];
    }
    if info.homepage.is_some() {
        item.homepage = info.homepage.clone();
    }
    if info.developer.is_some() {
        item.developer = info.developer.clone();
    }
}

fn read_metainfo(root: &Path, files: &[String], lang: Lang) -> Metainfo {
    files
        .iter()
        .find_map(|file| appstream::read(&root.join(file), lang.code()))
        .unwrap_or_default()
}

struct Context<'a> {
    places: &'a Places,
    icons: &'a IconIndex,
    records: &'a HashMap<String, i64>,
    lang: Lang,
}

pub fn run(places: &Places, lang: Lang) -> Catalog {
    // The Docker daemon measures what it keeps, which takes a while; it does so meanwhile.
    let docker = std::thread::spawn(|| capture("docker", &["system", "df", "-v", "--format", "{{json .}}"]));
    let running = usage::running_programs();
    let system = system::detect(places);
    let (packages, repositories) = match system {
        Some(PackageSystem::Pacman) => (
            pacman::read_all(&places.pacman_local, &running),
            pacman::parse_repositories(&capture("pacman", &["-Sl"])),
        ),
        Some(PackageSystem::Dpkg) => (
            dpkg::read_all(
                &places.dpkg_status,
                &places.apt_extended_states,
                &running,
                |names| {
                    let mut args = vec!["show", "--no-all-versions"];
                    args.extend(names.iter().map(String::as_str));
                    capture("apt-cache", &args)
                },
            ),
            dpkg::repositories(&capture("apt", &["list", "--installed"])),
        ),
        None => (Vec::new(), HashMap::new()),
    };
    let icons = IconIndex::build(places);
    let records = usage::plasma_launches(
        &places
            .home
            .join(".local/share/kactivitymanagerd/resources/database"),
    );
    let context = Context {
        places,
        icons: &icons,
        records: &records,
        lang,
    };

    let db = PackageDb::new(packages.iter().map(Package::node).collect());
    let mut owned_programs = HashMap::new();
    let mut owned_opt = HashSet::new();
    for package in &packages {
        for program in &package.files.programs {
            owned_programs.insert(program.clone(), package.name.clone());
        }
        owned_opt.extend(package.files.opt_roots.iter().cloned());
    }

    let owned_by_pacman = langs::Owned {
        node: packages
            .iter()
            .flat_map(|p| p.files.node_roots.iter().cloned())
            .collect(),
        python: packages
            .iter()
            .flat_map(|p| p.files.python_records.iter().cloned())
            .collect(),
    };
    let languages = langs::run(places, &owned_by_pacman, lang);
    let (cargo, claimed) = cargo_items(&context);
    let manual = manual_items(
        &claimed,
        &languages.claimed,
        &owned_programs,
        &owned_opt,
        &context,
    );

    let mut items: Vec<Item> = packages
        .iter()
        .map(|package| {
            let extra = manual.attached.get(&package.name).map_or(&[][..], Vec::as_slice);
            let mut item = package_item(
                package,
                system.unwrap_or(PackageSystem::Pacman),
                &repositories,
                extra,
                &context,
            );
            item.required_by = db.required_by(&package.name);
            item
        })
        .collect();
    items.extend(flatpak_items(&context));
    items.extend(steam_items(&context));
    items.extend(cargo);
    items.extend(languages.items);
    let docker = docker.join().unwrap_or_default();
    items.extend(docker::items(&docker::parse(&docker), lang.text()));
    items.extend(manual.items);
    let items = items.into_iter().map(Item::seal).collect();
    Catalog {
        items,
        db,
        system,
        theme_icons: theme_icons(&icons),
    }
}
