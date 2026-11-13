//! Builds the catalogue from every source. Runs on a background thread.

pub mod appstream;
pub mod desktop;
pub mod docker;
pub mod dpkg;
pub mod flatpak;
pub mod langs;
pub mod loose;
pub mod package;
pub mod pacman;
pub mod steam;
pub mod system;
pub mod usage;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::i18n::{Lang, Text};
use crate::icons::IconIndex;
use crate::model::{Access, Evidence, Item, Launch, PackageSystem, Removal, Section, Source, Usage};
use crate::plan::PackageDb;
use appstream::Metainfo;
use desktop::Launcher;
use package::Package;

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

fn package_item(
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

fn flatpak_items(context: &Context<'_>) -> Vec<Item> {
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

fn steam_items(context: &Context<'_>) -> Vec<Item> {
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

fn cargo_items(context: &Context<'_>) -> (Vec<Item>, HashSet<PathBuf>) {
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

fn manual_file(path: &Path, access: Access, text: &Text) -> Item {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let removal = Removal::Files {
        paths: vec![path.to_path_buf()],
        access,
    };
    let mut item = blank(format!("file:{}", path.display()), name, Source::Manual, removal);
    item.summary = path.display().to_string();
    item.description = (text.loose_file)(&path.display().to_string());
    item.size = Some(loose::size_of(path));
    item
}

/// Loose files, then user launchers: a launcher that starts one of those files joins it, so the
/// person sees one program, and removing it takes both.
fn manual_items(
    claimed: &HashSet<PathBuf>,
    claimed_folders: &[PathBuf],
    owned_programs: &HashMap<String, String>,
    owned_opt: &HashSet<String>,
    context: &Context<'_>,
) -> ManualScan {
    let (mut items, by_program) = user_files(claimed, claimed_folders, context);
    items.extend(system_files(owned_opt, context));
    let mut attached: HashMap<String, Vec<Launcher>> = HashMap::new();
    let folder = context.places.home.join(".local/share/applications");
    for path in loose::launchers_below(&folder) {
        let Some(launcher) = desktop::read(&path, context.lang) else {
            continue;
        };
        let exec = launcher.exec.clone().unwrap_or_default();
        if !launcher.shown || exec.contains("steam://") {
            continue;
        }
        let program = desktop::exec_program(&exec).unwrap_or_default();
        match owner_of(&program, owned_programs) {
            Some(owner) => attached.entry(owner.clone()).or_default().push(launcher),
            None => place_launcher(&mut items, &launcher, &path, &program, &by_program, context),
        }
    }
    ManualScan { items, attached }
}

/// Programs in the bin folders of the home that nothing else claims, and their file names.
fn user_files(
    claimed: &HashSet<PathBuf>,
    claimed_folders: &[PathBuf],
    context: &Context<'_>,
) -> (Vec<Item>, HashMap<String, PathBuf>) {
    let home = &context.places.home;
    let folders = [".local/bin", ".cargo/bin", "go/bin", ".deno/bin"].map(|folder| home.join(folder));
    let mut items = Vec::new();
    let mut by_program = HashMap::new();
    for path in folders.iter().flat_map(|dir| loose::files_in(dir)) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        let in_a_folder = claimed_folders.iter().any(|folder| canonical.starts_with(folder));
        if claimed.contains(&path) || in_a_folder {
            continue;
        }
        by_program.extend(file_name(&path).map(|name| (name, path.clone())));
        items.push(manual_file(&path, Access::User, context.lang.text()));
    }
    (items, by_program)
}

/// Programs in `/usr/local/bin`, and folders in `/opt` that no package owns.
fn system_files(owned_opt: &HashSet<String>, context: &Context<'_>) -> Vec<Item> {
    let text = context.lang.text();
    let programs = loose::files_in(&context.places.system_path("usr/local/bin"))
        .into_iter()
        .map(|path| manual_file(&path, Access::Root, text));
    let folders = loose::directories_in(&context.places.system_path("opt"))
        .into_iter()
        .filter(|path| !file_name(path).is_some_and(|name| owned_opt.contains(&name)))
        .map(|path| Item {
            section: Section::Utilities,
            ..manual_file(&path, Access::Root, text)
        });
    programs.chain(folders).collect()
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name().map(|name| name.to_string_lossy().into_owned())
}

/// The package that owns the program a launcher starts.
fn owner_of<'a>(program: &str, owned_programs: &'a HashMap<String, String>) -> Option<&'a String> {
    file_name(Path::new(program)).and_then(|name| owned_programs.get(&name))
}

/// A launcher of no package joins the loose file it starts, or becomes an entry of its own.
fn place_launcher(
    items: &mut Vec<Item>,
    launcher: &Launcher,
    path: &Path,
    program: &str,
    by_program: &HashMap<String, PathBuf>,
    context: &Context<'_>,
) {
    let target = loose::find_program(program, by_program).map(|file| format!("file:{}", file.display()));
    if let Some(item) = target.and_then(|id| items.iter_mut().find(|item| item.id == id)) {
        join_launcher(item, launcher, path, context);
        return;
    }
    let mut item = manual_file(path, Access::User, context.lang.text());
    item.description = (context.lang.text().loose_launcher)(&path.display().to_string());
    join_launcher(&mut item, launcher, path, context);
    items.push(item);
}

struct ManualScan {
    items: Vec<Item>,
    attached: HashMap<String, Vec<Launcher>>,
}

fn join_launcher(item: &mut Item, launcher: &Launcher, path: &Path, context: &Context<'_>) {
    let list = std::slice::from_ref(launcher);
    dress(item, list, &Metainfo::default(), context.icons);
    item.usage = usage_of(list, context.records);
    if let Removal::Files { paths, .. } = &mut item.removal
        && !paths.iter().any(|known| known == path)
    {
        paths.push(path.to_path_buf());
    }
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
