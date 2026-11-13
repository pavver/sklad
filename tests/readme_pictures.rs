//! The pictures of the README, drawn from an invented catalogue so that they show no real machine.
//! Run with `cargo test --test readme_pictures -- --ignored`.

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eframe::egui::accesskit::Role;
use eframe::egui::{self, Vec2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::{Harness, HarnessBuilder};
use sklad::i18n::{Choice, Lang};
use sklad::model::{
    Access, Evidence, Item, Launch, PackageSystem, Reason, Removal, Section, Source, SourceKind, Usage,
};
use sklad::plan::{Node, PackageDb};
use sklad::remove::{Backend, Event, Failure, Outcome, Spec};
use sklad::scan::Catalog;
use sklad::ui::{App, Scanner};

const DAY: i64 = 86_400;
const MEGABYTE: u64 = 1_048_576;
/// The margin the test harness leaves around the window, in pixels; a real window has none.
const HARNESS_EDGE: u32 = 8;

struct NoRemoval;

impl Backend for NoRemoval {
    fn preview(&self, _system: PackageSystem, names: &[String]) -> Result<Vec<String>, String> {
        Ok(names.to_vec())
    }

    fn run(&self, _spec: &Spec) -> Receiver<Event> {
        let (sender, receiver) = channel();
        let _ = sender.send(Event::Finished(Outcome::Failed(Failure::EmptyCommand)));
        receiver
    }
}

/// One invented entry.
struct Entry {
    title: &'static str,
    package: &'static str,
    about_en: &'static str,
    about_uk: &'static str,
    license: &'static str,
    site: &'static str,
    section: Section,
    source: Source,
    reason: Reason,
    megabytes: u64,
    usage: Usage,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs() as i64)
}

fn plasma(days: i64) -> Usage {
    Usage::Last {
        at: now() - days * DAY,
        by: Evidence::Plasma,
    }
}

fn extra() -> Source {
    Source::Repo("extra".to_owned())
}

fn program(title: &'static str, package: &'static str, about: [&'static str; 2]) -> Entry {
    Entry {
        title,
        package,
        about_en: about[0],
        about_uk: about[1],
        license: "GPL-3.0-or-later",
        site: "https://archlinux.org/packages/",
        section: Section::Utilities,
        source: extra(),
        reason: Reason::Explicit,
        megabytes: 120,
        usage: Usage::NoRecord,
    }
}

fn entries() -> Vec<Entry> {
    vec![
        Entry {
            section: Section::Graphics,
            megabytes: 1160,
            usage: plasma(420),
            ..program(
                "Blender",
                "blender",
                ["3D creation suite", "Набір для створення 3D"],
            )
        },
        Entry {
            section: Section::Graphics,
            megabytes: 412,
            usage: plasma(210),
            ..program("GIMP", "gimp", ["Image editor", "Редактор зображень"])
        },
        Entry {
            section: Section::Multimedia,
            megabytes: 96,
            source: Source::Flatpak,
            usage: plasma(96),
            ..program(
                "Audacity",
                "org.audacityteam.Audacity",
                ["Audio editor", "Звуковий редактор"],
            )
        },
        Entry {
            section: Section::Office,
            megabytes: 640,
            usage: plasma(34),
            license: "MPL-2.0",
            ..program(
                "LibreOffice",
                "libreoffice-fresh",
                ["Office suite", "Офісний пакет"],
            )
        },
        Entry {
            section: Section::Graphics,
            megabytes: 22,
            reason: Reason::Dependency,
            usage: plasma(12),
            ..program("Gwenview", "gwenview", ["Image viewer", "Переглядач зображень"])
        },
        Entry {
            section: Section::Office,
            megabytes: 18,
            reason: Reason::Dependency,
            usage: plasma(5),
            ..program("Okular", "okular", ["Document viewer", "Переглядач документів"])
        },
        Entry {
            section: Section::Development,
            megabytes: 390,
            source: Source::Aur,
            usage: plasma(1),
            license: "MIT",
            ..program("VSCodium", "vscodium-bin", ["Code editor", "Редактор коду"])
        },
        Entry {
            section: Section::Multimedia,
            megabytes: 75,
            usage: Usage::NoRecord,
            ..program("VLC", "vlc", ["Media player", "Медіапрогравач"])
        },
        Entry {
            section: Section::Internet,
            megabytes: 260,
            usage: Usage::Running,
            license: "MPL-2.0",
            ..program("Firefox", "firefox", ["Web browser", "Вебпереглядач"])
        },
    ]
}

fn item_of(entry: Entry, lang: Lang) -> Item {
    // The README has English and Ukrainian pictures only.
    let about = match lang {
        Lang::Uk => entry.about_uk,
        _ => entry.about_en,
    };
    let removal = match entry.source {
        Source::Flatpak => Removal::Flatpak {
            app_id: entry.package.to_owned(),
            scope: sklad::model::FlatpakScope::User,
        },
        _ => Removal::Package(PackageSystem::Pacman),
    };
    Item {
        id: format!("pacman:{}", entry.package),
        title: entry.title.to_owned(),
        package: entry.package.to_owned(),
        version: "1.0.0-1".to_owned(),
        summary: about.to_owned(),
        description: about.to_owned(),
        licenses: vec![entry.license.to_owned()],
        homepage: Some(entry.site.to_owned()),
        developer: None,
        source: entry.source,
        section: entry.section,
        reason: Some(entry.reason),
        size: Some(entry.megabytes * MEGABYTE),
        installed: Some(now() - 500 * DAY),
        usage: entry.usage,
        icon: None,
        launchers: Vec::new(),
        launch: Some(Launch::Desktop(
            format!("/usr/share/applications/{}.desktop", entry.package).into(),
        )),
        groups: Vec::new(),
        depends: Vec::new(),
        required_by: Vec::new(),
        removal,
        warning: None,
        haystack: String::new(),
    }
    .seal()
}

fn toolchain(title: &str, name: &str, version: &str, lang: Lang) -> Item {
    let text = lang.text();
    let argv = ["rustup", "toolchain", "uninstall", name]
        .map(str::to_owned)
        .to_vec();
    Item {
        id: format!("rustup:{name}"),
        title: title.to_owned(),
        package: name.to_owned(),
        version: version.to_owned(),
        summary: text.rust_toolchain.to_owned(),
        description: text.rust_toolchain.to_owned(),
        licenses: Vec::new(),
        homepage: Some("https://www.rust-lang.org".to_owned()),
        developer: None,
        source: Source::Rustup,
        section: Section::Console,
        reason: None,
        size: Some(2100 * MEGABYTE),
        installed: None,
        usage: Usage::NotTracked,
        icon: None,
        launchers: Vec::new(),
        launch: None,
        groups: Vec::new(),
        depends: Vec::new(),
        required_by: Vec::new(),
        removal: Removal::Manager {
            name: "rustup".to_owned(),
            argv,
            access: Access::User,
        },
        warning: None,
        haystack: String::new(),
    }
    .seal()
}

fn node(name: &str, depends: &[&str], reason: Reason, meta: bool) -> Node {
    Node {
        name: name.to_owned(),
        depends: depends.iter().map(|dep| (*dep).to_owned()).collect(),
        recommends: Vec::new(),
        provides: Vec::new(),
        reason,
        critical: false,
        meta,
        size: 22 * MEGABYTE,
    }
}

fn demo(lang: Lang) -> Catalog {
    let mut items: Vec<Item> = entries().into_iter().map(|entry| item_of(entry, lang)).collect();
    for item in items
        .iter_mut()
        .filter(|item| item.reason == Some(Reason::Dependency))
    {
        item.required_by = vec!["kde-graphics-meta".to_owned()];
    }
    let mut stable = toolchain("Rust stable", "stable-x86_64-unknown-linux-gnu", "1.99.1", lang);
    stable.warning = Some(lang.text().default_toolchain.to_owned());
    items.push(stable);
    items.push(toolchain(
        "Rust nightly",
        "nightly-x86_64-unknown-linux-gnu",
        "1.101.0-nightly",
        lang,
    ));
    let mut nodes: Vec<Node> = [
        "blender",
        "gimp",
        "libreoffice-fresh",
        "vscodium-bin",
        "vlc",
        "firefox",
    ]
    .iter()
    .map(|name| node(name, &[], Reason::Explicit, false))
    .collect();
    nodes.push(node(
        "kde-graphics-meta",
        &["gwenview", "okular"],
        Reason::Explicit,
        true,
    ));
    nodes.push(node("gwenview", &[], Reason::Dependency, false));
    nodes.push(node("okular", &[], Reason::Dependency, false));
    Catalog {
        items,
        db: PackageDb::new(nodes),
        system: Some(PackageSystem::Pacman),
    }
}

fn settle(harness: &mut Harness<'_, App>) {
    for _ in 0..30 {
        harness.step();
        sleep(Duration::from_millis(5));
    }
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    settle(harness);
    let target = [Role::Button, Role::Row, Role::Switch]
        .into_iter()
        .find_map(|role| {
            harness
                .query_all(By::new().role(role).label_contains(label))
                .next()
        });
    if let Some(button) = target {
        button.click();
    }
    settle(harness);
}

/// The sources lie at the end of the side bar, below the sections.
fn scroll_side_bar_to_the_end(harness: &mut Harness<'_, App>) {
    harness.event(egui::Event::PointerMoved(egui::pos2(120.0, 500.0)));
    for _ in 0..20 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -200.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
            source: egui::MouseWheelSource::Wheel,
        });
        harness.step();
    }
    settle(harness);
}

/// Takes the pointer out of the window first: it would light up a row and show on the picture.
fn save(harness: &mut Harness<'_, App>, name: &str) -> Result<(), String> {
    harness.event(egui::Event::PointerGone);
    settle(harness);
    let folder = Path::new("docs/screenshots");
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let image = harness.render().map_err(|error| error.to_string())?;
    let (width, height) = image.dimensions();
    let inner = image::imageops::crop_imm(
        &image,
        HARNESS_EDGE,
        HARNESS_EDGE,
        width - 2 * HARNESS_EDGE,
        height - 2 * HARNESS_EDGE,
    );
    inner
        .to_image()
        .save(folder.join(name))
        .map_err(|error| error.to_string())
}

fn window<'a>(lang: Lang) -> Harness<'a, App> {
    let scanner: Scanner = Arc::new(demo);
    HarnessBuilder::default()
        .with_size(Vec2::new(1376.0, 1056.0))
        .with_max_steps(10_000)
        .wgpu()
        .build_ui_state(|ui, app: &mut App| app.draw(ui), {
            let context = egui::Context::default();
            context.set_theme(egui::Theme::Dark);
            App::new(&context, Arc::new(NoRemoval), scanner, Choice::fixed(lang))
        })
}

fn pictures(lang: Lang) -> Result<(), String> {
    let text = lang.text();
    let code = lang.code();
    let mut harness = window(lang);
    click(&mut harness, "GIMP");
    save(&mut harness, &format!("{code}-catalogue.png"))?;

    let mut harness = window(lang);
    click(&mut harness, "Gwenview");
    click(&mut harness, text.remove);
    save(&mut harness, &format!("{code}-removal.png"))?;

    let mut harness = window(lang);
    click(&mut harness, text.dark_theme);
    scroll_side_bar_to_the_end(&mut harness);
    click(&mut harness, (text.source)(SourceKind::Rustup));
    click(&mut harness, "Rust stable");
    save(&mut harness, &format!("{code}-tools.png"))
}

#[test]
#[ignore = "writes the pictures of the README"]
fn pictures_of_the_readme() -> Result<(), String> {
    pictures(Lang::En)?;
    pictures(Lang::Uk)
}
