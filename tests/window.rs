//! The window driven through real frames: clicks go through the pointer, not through handlers.
//! The backend is a stand-in, so no test here can touch a real package.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use eframe::egui::Vec2;
use eframe::egui::accesskit::Role;
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::{Harness, HarnessBuilder};
use sklad::i18n::{Choice, Lang};
use sklad::model::{Evidence, Item, Launch, PackageSystem, Reason, Removal, Section, Source, Usage};
use sklad::plan::{Node, PackageDb};
use sklad::remove::{Backend, Event, Failure, Outcome, Spec};
use sklad::scan::Catalog;
use sklad::ui::{App, Scanner};

const DAY: i64 = 86_400;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_secs() as i64)
}

struct FakeBackend {
    preview: Mutex<Option<Result<Vec<String>, String>>>,
    outcome: Outcome,
    ran: Mutex<Vec<Spec>>,
}

impl FakeBackend {
    fn new(outcome: Outcome) -> Arc<Self> {
        Arc::new(Self {
            preview: Mutex::new(None),
            outcome,
            ran: Mutex::new(Vec::new()),
        })
    }

    fn refusing(message: &str) -> Arc<Self> {
        let backend = Self::new(Outcome::Done);
        if let Ok(mut preview) = backend.preview.lock() {
            *preview = Some(Err(message.to_owned()));
        }
        backend
    }

    fn commands(&self) -> Vec<Spec> {
        self.ran.lock().map(|ran| ran.clone()).unwrap_or_default()
    }
}

impl Backend for FakeBackend {
    fn preview(&self, names: &[String]) -> Result<Vec<String>, String> {
        let forced = self.preview.lock().ok().and_then(|preview| preview.clone());
        forced.unwrap_or_else(|| Ok(names.to_vec()))
    }

    fn run(&self, spec: &Spec) -> Receiver<Event> {
        if let Ok(mut ran) = self.ran.lock() {
            ran.push(spec.clone());
        }
        let (sender, receiver) = channel();
        let _ = sender.send(Event::Line("checking dependencies...".to_owned()));
        let _ = sender.send(Event::Finished(self.outcome.clone()));
        receiver
    }
}

fn item(package: &str, title: &str, section: Section, usage: Usage) -> Item {
    Item {
        id: format!("pacman:{package}"),
        title: title.to_owned(),
        package: package.to_owned(),
        version: "1.2-1".to_owned(),
        summary: format!("Опис {title}"),
        description: format!("Докладний опис {title}."),
        licenses: vec!["GPL-3.0".to_owned()],
        homepage: Some("https://example.org".to_owned()),
        developer: Some("Розробник".to_owned()),
        source: Source::Repo("extra".to_owned()),
        section,
        reason: Some(Reason::Explicit),
        size: Some(52_428_800),
        installed: Some(now() - 200 * DAY),
        usage,
        icon: None,
        launchers: Vec::new(),
        launch: None,
        groups: Vec::new(),
        depends: Vec::new(),
        required_by: Vec::new(),
        removal: Removal::Pacman,
        warning: None,
        haystack: String::new(),
    }
    .seal()
}

fn node(name: &str, depends: &[&str], reason: Reason, meta: bool) -> Node {
    Node {
        name: name.to_owned(),
        depends: depends.iter().map(|dep| (*dep).to_owned()).collect(),
        provides: Vec::new(),
        reason,
        meta,
        size: 1_048_576,
    }
}

/// A system with a metapackage, a library programs need, and a critical package.
fn catalog() -> Catalog {
    let plasma = Evidence::Plasma;
    let mut lib = item("libshared", "Бібліотека", Section::Libraries, Usage::NotTracked);
    lib.required_by = vec!["editor".to_owned()];
    let mut player = item("player", "Програвач", Section::Multimedia, Usage::NoRecord);
    player.reason = Some(Reason::Dependency);
    let mut home_tool = item("tool", "Мій скрипт", Section::Utilities, Usage::NotTracked);
    home_tool.id = "file:/home/me/.local/bin/tool".to_owned();
    home_tool.source = Source::Manual;
    home_tool.removal = Removal::Files {
        paths: vec![PathBuf::from("/home/me/.local/bin/tool")],
        access: sklad::model::Access::User,
    };
    let mut system_tool = item("opt-app", "Програма в opt", Section::Utilities, Usage::NotTracked);
    system_tool.id = "file:/opt/opt-app".to_owned();
    system_tool.source = Source::Manual;
    system_tool.removal = Removal::Files {
        paths: vec![PathBuf::from("/opt/opt-app")],
        access: sklad::model::Access::Root,
    };
    let mut toolchain = item("stable", "Rust stable", Section::Development, Usage::NotTracked);
    toolchain.id = "rustup:stable".to_owned();
    toolchain.source = Source::Rustup;
    toolchain.warning = Some("Це типовий інструментарій.".to_owned());
    toolchain.removal = Removal::Manager {
        name: "rustup".to_owned(),
        argv: ["rustup", "toolchain", "uninstall", "stable"]
            .map(str::to_owned)
            .to_vec(),
        access: sklad::model::Access::User,
    };
    let mut leftover = item("pyzfs", "pyzfs", Section::Development, Usage::NotTracked);
    leftover.id = "python:pyzfs".to_owned();
    leftover.source = Source::Python;
    leftover.removal = Removal::Unavailable {
        reason: "Запис поза менеджером пакетів.".to_owned(),
    };
    Catalog {
        items: vec![
            player,
            toolchain,
            leftover,
            item(
                "editor",
                "Редактор",
                Section::Office,
                Usage::Last {
                    at: now() - 3 * DAY,
                    by: plasma,
                },
            ),
            lib,
            item("systemd-app", "Сервіс", Section::System, Usage::Running),
            home_tool,
            system_tool,
        ],
        db: PackageDb::new(vec![
            node("bundle-meta", &["player", "viewer"], Reason::Explicit, true),
            node("player", &[], Reason::Dependency, false),
            node("viewer", &[], Reason::Dependency, false),
            node("editor", &["libshared"], Reason::Explicit, false),
            node("libshared", &[], Reason::Explicit, false),
            node("systemd-app", &["systemd"], Reason::Explicit, false),
            node("systemd", &[], Reason::Dependency, false),
        ]),
        system: Some(PackageSystem::Pacman),
    }
}

fn window<'a>(backend: Arc<FakeBackend>) -> Harness<'a, App> {
    window_of_size(backend, Vec2::new(1360.0, 1200.0))
}

fn window_of_size<'a>(backend: Arc<FakeBackend>, size: Vec2) -> Harness<'a, App> {
    window_speaking(backend, size, Arc::new(|_| catalog()), Lang::Uk)
}

fn window_speaking<'a>(
    backend: Arc<FakeBackend>,
    size: Vec2,
    scanner: Scanner,
    lang: Lang,
) -> Harness<'a, App> {
    HarnessBuilder::default()
        .with_size(size)
        .with_max_steps(10_000)
        .wgpu()
        .build_ui_state(|ui, app: &mut App| app.draw(ui), {
            let context = eframe::egui::Context::default();
            context.set_theme(eframe::egui::Theme::Dark);
            App::new(&context, backend, scanner, Choice::fixed(lang))
        })
}

/// Steps real frames while background work finishes. `run` would never return during a pending
/// state, which repaints on purpose.
fn settle(harness: &mut Harness<'_, App>) {
    for _ in 0..30 {
        harness.step();
        sleep(Duration::from_millis(5));
    }
}

fn shot(harness: &mut Harness<'_, App>, name: &str) {
    let folder = PathBuf::from("target/shots");
    let _ = std::fs::create_dir_all(&folder);
    match harness.render() {
        Ok(image) => {
            if let Err(error) = image.save(folder.join(format!("{name}.png"))) {
                println!("cannot save {name}: {error}");
            }
        }
        Err(error) => println!("cannot render {name}: {error}"),
    }
}

fn open_removal(harness: &mut Harness<'_, App>, title: &str) {
    settle(harness);
    harness.get_by_role_and_label(Role::Row, title).click();
    settle(harness);
    harness
        .get(By::new().role(Role::Button).label_contains("Видалити"))
        .click();
    settle(harness);
}

fn has_row(harness: &Harness<'_, App>, title: &str) -> bool {
    harness.query_by_role_and_label(Role::Row, title).is_some()
}

fn confirm(harness: &mut Harness<'_, App>) {
    let button = harness
        .get_all(By::new().role(Role::Button).label_contains("Видалити"))
        .last();
    if let Some(button) = button {
        button.click();
    }
    settle(harness);
}

#[test]
fn the_window_opens_with_what_was_unused_longest_on_top() {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    shot(&mut harness, "01-list");
    let first = harness.get_by_role_and_label(Role::Row, "Програвач").rect().top();
    let later = harness.get_by_role_and_label(Role::Row, "Редактор").rect().top();
    assert!(first < later, "a program with no record of a launch goes first");
}

#[test]
fn a_click_opens_the_card() {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "Редактор").click();
    settle(&mut harness);
    shot(&mut harness, "02-card");
    assert!(harness.query_by_label("GPL-3.0").is_some());
    assert!(harness.query_by_label("Ліцензія").is_some());
}

#[test]
fn a_program_out_of_a_metapackage_is_removed_with_its_siblings_kept() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Програвач");
    shot(&mut harness, "03-metapackage");
    assert!(harness.query_by_label_contains("метапакет bundle-meta").is_some());
    confirm(&mut harness);
    let commands = backend.commands();
    assert_eq!(commands.len(), 1, "exactly one command ran");
    assert_eq!(commands[0].argv[0], "pkexec");
    assert_eq!(
        commands[0].argv[3],
        "pacman -D --asexplicit viewer && pacman -Rn --noconfirm bundle-meta player"
    );
}

#[test]
fn a_library_programs_need_cannot_be_removed_until_that_is_chosen() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window(Arc::clone(&backend));
    settle(&mut harness);
    harness
        .get(By::new().role(Role::Button).label_contains("Бібліотеки"))
        .click();
    open_removal(&mut harness, "Бібліотека");
    shot(&mut harness, "04-blocked");
    assert!(
        harness
            .query_by_label_contains("потрібен програмам: editor")
            .is_some()
    );
    confirm(&mut harness);
    assert!(
        backend.commands().is_empty(),
        "the button does nothing while blocked"
    );

    harness
        .get_by_role_and_label(Role::Tab, "Видалити разом із ними")
        .click();
    settle(&mut harness);
    shot(&mut harness, "05-with-dependents");
    confirm(&mut harness);
    let commands = backend.commands();
    assert_eq!(commands.len(), 1);
    assert!(commands[0].argv[3].contains("editor libshared"));
}

#[test]
fn a_critical_package_needs_a_second_confirmation() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Сервіс");
    shot(&mut harness, "06-critical");
    assert!(
        harness
            .query_by_label_contains("система може не запуститися: systemd")
            .is_some()
    );
    assert!(harness.query_by_label_contains("зараз працює").is_some());
    confirm(&mut harness);
    assert!(backend.commands().is_empty(), "not without the box ticked");

    harness.get_by_label("Я розумію наслідки").click();
    settle(&mut harness);
    confirm(&mut harness);
    assert_eq!(backend.commands().len(), 1);
}

#[test]
fn nothing_runs_when_pacman_refuses_the_list() {
    let backend = FakeBackend::refusing("breaks dependency");
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Редактор");
    shot(&mut harness, "07-pacman-refused");
    assert!(harness.query_by_label_contains("pacman відмовив").is_some());
    confirm(&mut harness);
    assert!(backend.commands().is_empty());
}

#[test]
fn nothing_runs_when_pacman_would_remove_something_else() {
    let backend = FakeBackend::new(Outcome::Done);
    if let Ok(mut preview) = backend.preview.lock() {
        *preview = Some(Ok(vec!["editor".to_owned(), "surprise".to_owned()]));
    }
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Редактор");
    assert!(harness.query_by_label_contains("інший список").is_some());
    confirm(&mut harness);
    assert!(backend.commands().is_empty());
}

#[test]
fn a_closed_password_window_changes_nothing_and_says_so() {
    let backend = FakeBackend::new(Outcome::Dismissed);
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Редактор");
    confirm(&mut harness);
    assert!(harness.query_by_label_contains("Пароль не введено").is_some());
}

#[test]
fn a_failure_keeps_the_output_on_screen() {
    let backend = FakeBackend::new(Outcome::Failed(Failure::Code(1)));
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Редактор");
    confirm(&mut harness);
    shot(&mut harness, "08-failed");
    assert!(harness.query_by_label_contains("Не вдалося видалити").is_some());
    assert!(harness.query_by_label("checking dependencies...").is_some());
}

#[test]
fn success_closes_the_dialog_and_reads_the_system_again() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Редактор");
    let button = harness
        .get_all(By::new().role(Role::Button).label_contains("Видалити"))
        .last();
    if let Some(button) = button {
        button.click();
    }
    // The toast lives a few seconds, and every frame of the harness moves its clock on, so it is
    // looked for frame by frame rather than after the window settles.
    let mut toasted = false;
    for _ in 0..40 {
        harness.step();
        sleep(Duration::from_millis(20));
        if harness.query_by_label_contains("Видалено: Редактор").is_some() {
            toasted = true;
            break;
        }
    }
    harness.step();
    shot(&mut harness, "12-removed");
    assert!(toasted, "a toast says what was removed");
    assert!(
        harness.query_by_label("Скасувати").is_none(),
        "the dialog is gone"
    );
}

#[test]
fn user_files_go_to_the_bin_without_a_password() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Мій скрипт");
    confirm(&mut harness);
    let commands = backend.commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].argv[..2], ["gio".to_owned(), "trash".to_owned()]);
}

#[test]
fn a_toolchain_is_removed_by_its_manager_after_the_warning() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Rust stable");
    shot(&mut harness, "20-toolchain-dialog");
    assert!(
        harness
            .query_by_label_contains("Це типовий інструментарій.")
            .is_some()
    );
    confirm(&mut harness);
    let commands = backend.commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].argv, ["rustup", "toolchain", "uninstall", "stable"]);
}

#[test]
fn a_leftover_says_why_and_cannot_be_removed() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window(Arc::clone(&backend));
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "pyzfs").click();
    settle(&mut harness);
    shot(&mut harness, "21-leftover-card");
    assert!(
        harness
            .query_by_label_contains("Запис поза менеджером пакетів.")
            .is_some()
    );
    let button = harness.get(By::new().role(Role::Button).label_contains("Видалити"));
    assert!(button.accesskit_node().is_disabled());
    button.click();
    settle(&mut harness);
    assert!(harness.query_by_label_contains("Буде видалено").is_none());
    assert!(backend.commands().is_empty());
}

/// The fixture with its Ukrainian content replaced, so that only the interface can bring a
/// Ukrainian word into the window.
fn plain_catalog() -> Catalog {
    let mut plain = catalog();
    for item in &mut plain.items {
        item.title = item.package.clone();
        item.summary = "About it".to_owned();
        item.description = "A longer text about it.".to_owned();
        item.developer = Some("Someone".to_owned());
        item.warning = None;
        if let Removal::Unavailable { reason } = &mut item.removal {
            *reason = "A reason".to_owned();
        }
    }
    plain
}

fn ukrainian_labels(harness: &Harness<'_, App>) -> Vec<String> {
    let ukrainian = |text: &str| {
        text.chars()
            .any(|symbol| ('\u{0400}'..='\u{04FF}').contains(&symbol))
    };
    let found = harness.query_all(By::new().predicate(move |node| {
        let label = node.label().unwrap_or_default();
        ukrainian(&label) && !label.contains("Українська")
    }));
    found
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

#[test]
fn the_english_window_has_no_ukrainian_word_in_any_state() {
    let size = Vec2::new(1360.0, 860.0);
    let scanner: Scanner = Arc::new(|_| plain_catalog());
    let mut harness = window_speaking(FakeBackend::new(Outcome::Done), size, scanner, Lang::En);
    settle(&mut harness);
    assert!(
        harness
            .query(By::new().role(Role::Button).label_contains("Refresh"))
            .is_some()
    );
    assert_eq!(ukrainian_labels(&harness), Vec::<String>::new());
    harness.get_by_role_and_label(Role::Row, "editor").click();
    settle(&mut harness);
    shot(&mut harness, "30-english-card");
    assert_eq!(ukrainian_labels(&harness), Vec::<String>::new());
    harness
        .get(By::new().role(Role::Button).label_contains("Remove"))
        .click();
    settle(&mut harness);
    shot(&mut harness, "31-english-dialog");
    assert!(harness.query_by_label_contains("will be removed").is_some());
    assert_eq!(ukrainian_labels(&harness), Vec::<String>::new());
}

#[test]
fn the_switch_changes_the_words_and_reads_the_catalogue_in_the_new_language() {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&asked);
    let scanner: Scanner = Arc::new(move |lang| {
        if let Ok(mut log) = log.lock() {
            log.push(lang);
        }
        catalog()
    });
    let size = Vec2::new(1360.0, 860.0);
    let mut harness = window_speaking(FakeBackend::new(Outcome::Done), size, scanner, Lang::Uk);
    settle(&mut harness);
    assert!(
        harness
            .query(By::new().role(Role::Button).label_contains("Оновити"))
            .is_some()
    );
    harness
        .get(By::new().role(Role::Tab).label_contains("English"))
        .click();
    settle(&mut harness);
    assert!(
        harness
            .query(By::new().role(Role::Button).label_contains("Refresh"))
            .is_some()
    );
    assert!(
        harness
            .query(By::new().role(Role::Button).label_contains("Оновити"))
            .is_none()
    );
    assert_eq!(
        asked.lock().map(|log| log.clone()).ok(),
        Some(vec![Lang::Uk, Lang::En])
    );
    harness
        .get(By::new().role(Role::Tab).label_contains("Українська"))
        .click();
    settle(&mut harness);
    assert!(
        harness
            .query(By::new().role(Role::Button).label_contains("Оновити"))
            .is_some()
    );
}

#[test]
fn files_outside_the_home_are_confirmed_twice() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window(Arc::clone(&backend));
    open_removal(&mut harness, "Програма в opt");
    shot(&mut harness, "09-irreversible");
    assert!(
        harness
            .query_by_label_contains("буде видалено безповоротно")
            .is_some()
    );
    confirm(&mut harness);
    assert!(backend.commands().is_empty());
}

#[test]
fn the_list_stays_inside_its_column_when_the_card_is_open() {
    for width in [1100.0, 1360.0, 1900.0] {
        let backend = FakeBackend::new(Outcome::Done);
        let mut harness = window_of_size(backend, Vec2::new(width, 700.0));
        settle(&mut harness);
        harness.get_by_role_and_label(Role::Row, "Редактор").click();
        settle(&mut harness);
        shot(&mut harness, &format!("10-width-{width}"));
        let row = harness.get_by_role_and_label(Role::Row, "Редактор").rect();
        let card = harness.get_by_label("Ліцензія").rect();
        assert!(
            row.right() <= card.left(),
            "at {width}: row ends at {}, card at {}",
            row.right(),
            card.left()
        );
    }
}

#[test]
fn search_narrows_the_list() {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    let field = harness.get_by_role(Role::TextInput);
    field.focus();
    field.type_text("редактор");
    settle(&mut harness);
    assert!(has_row(&harness, "Редактор"));
    assert!(!has_row(&harness, "Програвач"));
}

/// Real data, real icons, the real `pacman -Rp`. Removal itself stays impossible: `run` only fails.
struct ReadOnlyBackend;

impl Backend for ReadOnlyBackend {
    fn preview(&self, names: &[String]) -> Result<Vec<String>, String> {
        sklad::remove::SystemBackend.preview(names)
    }

    fn run(&self, _spec: &Spec) -> Receiver<Event> {
        let (sender, receiver) = channel();
        let _ = sender.send(Event::Finished(Outcome::Failed(Failure::NotStarted(
            "read-only test".to_owned(),
        ))));
        receiver
    }
}

#[test]
#[ignore = "reads the live system"]
fn pictures_of_the_real_system() {
    let scanner: Scanner = Arc::new(|lang| sklad::scan::run(&sklad::scan::Places::system(), lang));
    let context = eframe::egui::Context::default();
    let app = App::new(
        &context,
        Arc::new(ReadOnlyBackend),
        scanner,
        Choice::fixed(Lang::Uk),
    );
    let mut harness = HarnessBuilder::default()
        .with_size(Vec2::new(1500.0, 950.0))
        .with_max_steps(100_000)
        .allow_missing_glyphs()
        .wgpu()
        .build_ui_state(|ui, app: &mut App| app.draw(ui), app);
    for _ in 0..3000 {
        harness.step();
        sleep(Duration::from_millis(5));
        if harness
            .query_all(By::new().label_contains("записи"))
            .next()
            .is_some()
        {
            break;
        }
    }
    for _ in 0..200 {
        harness.step();
        sleep(Duration::from_millis(5));
    }
    shot(&mut harness, "real-01-apps");
    harness
        .get(By::new().role(Role::Button).label_contains("Ігри"))
        .click();
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Button, "Розмір").click();
    settle(&mut harness);
    for _ in 0..100 {
        harness.step();
        sleep(Duration::from_millis(5));
    }
    shot(&mut harness, "real-02-games");
    harness
        .get(By::new().role(Role::Button).label_contains("Усі програми"))
        .click();
    settle(&mut harness);
    let field = harness.get_by_role(Role::TextInput);
    field.focus();
    field.type_text("kate");
    for _ in 0..80 {
        harness.step();
        sleep(Duration::from_millis(5));
    }
    let row = harness
        .get_all(By::new().role(Role::Row).label_contains("Kate"))
        .next();
    if let Some(row) = row {
        row.click();
    }
    for _ in 0..100 {
        harness.step();
        sleep(Duration::from_millis(5));
    }
    shot(&mut harness, "real-03-card");
    harness
        .get(By::new().role(Role::Button).label_contains("Видалити"))
        .click();
    for _ in 0..200 {
        harness.step();
        sleep(Duration::from_millis(5));
    }
    shot(&mut harness, "real-04-dialog");
    harness.key_press(eframe::egui::Key::Escape);
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Switch, "Темна тема").click();
    settle(&mut harness);
    shot(&mut harness, "real-05-light");
}

/// Close-up pictures at the scale of the real screen, to judge alignment by eye.
#[test]
#[ignore = "pictures only"]
fn close_up_pictures() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window_of_size(Arc::clone(&backend), Vec2::new(1920.0, 1040.0));
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "Сервіс").click();
    settle(&mut harness);
    shot(&mut harness, "zoom-main");
    harness
        .get(By::new().role(Role::Button).label_contains("Видалити"))
        .click();
    settle(&mut harness);
    shot(&mut harness, "zoom-dialog");
}

/// Top and bottom of what is drawn inside a rectangle, found by the pixels that differ from the
/// colour of its middle-right area, which is plain background.
fn ink_rows(image: &image::RgbaImage, rect: eframe::egui::Rect) -> Option<(f32, f32)> {
    let left = rect.left() as u32 + 4;
    let right = rect.right() as u32 - 4;
    let top = rect.top() as u32 + 3;
    let bottom = rect.bottom() as u32 - 3;
    let background = *image.get_pixel(right - 2, top);
    let differs = |pixel: &image::Rgba<u8>| pixel.0.iter().zip(background.0).any(|(a, b)| a.abs_diff(b) > 40);
    let rows: Vec<u32> = (top..bottom)
        .filter(|&y| (left..right).any(|x| differs(image.get_pixel(x, y))))
        .collect();
    Some((*rows.first()? as f32, *rows.last()? as f32 + 1.0))
}

#[test]
fn the_search_hint_sits_in_the_middle_of_its_field() -> Result<(), String> {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    let field = harness.get_by_role(Role::TextInput).rect();
    let image = harness.render()?;
    let (top, bottom) = ink_rows(&image, field).ok_or("nothing drawn in the field")?;
    let offset = (top + bottom) / 2.0 - field.center().y;
    assert!(
        offset.abs() <= 1.5,
        "text centre is {offset} px off, ink {top}..{bottom}, field {field:?}"
    );
    Ok(())
}

#[test]
fn controls_of_one_bar_share_their_top_and_height() {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    let refresh = harness
        .get(By::new().role(Role::Button).label_contains("Оновити"))
        .rect();
    let search = harness.get_by_role(Role::TextInput).rect();
    let origin = harness.get_by_role_and_label(Role::Tab, "Непотрібні").rect();
    assert_eq!(refresh.height(), 32.0, "a toolbar command is 8 DU");
    assert_eq!(refresh.width(), refresh.height(), "an icon button is square");
    for other in [search, origin] {
        assert_eq!(other.center().y, refresh.center().y);
    }
    assert_eq!(
        origin.height(),
        refresh.height() - 4.0,
        "a segment sits inside its 0.5 DU padding"
    );
}

/// Runs frames until the frame output holds a command the test is after.
fn wait_for_command(
    harness: &mut Harness<'_, App>,
    wanted: impl Fn(&eframe::egui::OutputCommand) -> bool,
) -> bool {
    for _ in 0..30 {
        harness.step();
        if harness.output().platform_output.commands.iter().any(&wanted) {
            return true;
        }
    }
    false
}

#[test]
fn the_card_opens_the_site_and_copies_the_command() {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "Редактор").click();
    settle(&mut harness);
    harness
        .get(By::new().role(Role::Button).label_contains("Сайт"))
        .click();
    assert!(wait_for_command(&mut harness, |command| {
        matches!(command, eframe::egui::OutputCommand::OpenUrl(open) if open.url == "https://example.org")
    }));
    harness
        .get(By::new().role(Role::Button).label_contains("Скопіювати"))
        .click();
    assert!(wait_for_command(&mut harness, |command| {
        matches!(command, eframe::egui::OutputCommand::CopyText(text) if text == "sudo pacman -Rn editor")
    }));
}

/// Without these features eframe drops copy and open-link requests without a word.
#[test]
fn the_window_library_is_built_with_clipboard_and_links() -> Result<(), std::io::Error> {
    let manifest = std::fs::read_to_string("Cargo.toml")?;
    let line = manifest
        .lines()
        .find(|line| line.starts_with("eframe"))
        .unwrap_or_default();
    for feature in ["\"clipboard\"", "\"links\"", "\"accesskit\""] {
        assert!(line.contains(feature), "eframe lacks {feature}");
    }
    Ok(())
}

#[test]
fn a_copy_button_confirms_the_copy() -> Result<(), String> {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "Редактор").click();
    settle(&mut harness);
    let button = || By::new().role(Role::Button).label_contains("Скопіювати");
    let rect = harness.get(button()).rect();
    harness.hover_at(eframe::egui::pos2(700.0, 500.0));
    settle(&mut harness);
    let before = fill_of(&harness.render()?, rect);
    harness.get(button()).click();
    harness.step();
    harness.step();
    harness.hover_at(eframe::egui::pos2(700.0, 500.0));
    harness.step();
    let flashing = fill_of(&harness.render()?, rect);
    shot(&mut harness, "11-copied");
    assert_ne!(before, flashing, "the button turns to its success colour");
    sleep(Duration::from_millis(1400));
    settle(&mut harness);
    assert_eq!(
        before,
        fill_of(&harness.render()?, rect),
        "and goes back by itself"
    );
    Ok(())
}

fn fill_of(image: &image::RgbaImage, rect: eframe::egui::Rect) -> image::Rgba<u8> {
    *image.get_pixel(rect.left() as u32 + 3, rect.top() as u32 + 3)
}

#[test]
fn every_kind_of_button_reacts_to_the_pointer() -> Result<(), String> {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "Редактор").click();
    settle(&mut harness);
    let targets = [
        (Role::Button, "Оновити"),
        (Role::Button, "Сайт"),
        (Role::Button, "Видалити"),
        (Role::Button, "Мультимедіа"),
        (Role::Row, "Сервіс"),
        (Role::Tab, "Непотрібні"),
    ];
    let mut still = Vec::new();
    for (role, target) in targets {
        harness.hover_at(eframe::egui::pos2(700.0, 500.0));
        settle(&mut harness);
        let rect = harness.get(By::new().role(role).label_contains(target)).rect();
        let before = fill_of(&harness.render()?, rect);
        harness.hover_at(rect.center());
        settle(&mut harness);
        let after = fill_of(&harness.render()?, rect);
        if before == after {
            still.push(target);
        }
    }
    assert!(still.is_empty(), "no reaction to the pointer: {still:?}");
    Ok(())
}

/// Enough programs to scroll.
fn long_catalog() -> Catalog {
    let items: Vec<Item> = (0..60)
        .map(|index| {
            let mut item = item(
                &format!("p{index:02}"),
                &format!("Програма {index:02}"),
                Section::Office,
                Usage::NoRecord,
            );
            item.id = format!("pacman:p{index:02}");
            item
        })
        .collect();
    let nodes = items
        .iter()
        .map(|item| node(&item.package, &[], Reason::Explicit, false))
        .collect();
    Catalog {
        items,
        db: PackageDb::new(nodes),
        system: Some(PackageSystem::Pacman),
    }
}

/// The label of the highest row shown below the header.
fn top_row(harness: &Harness<'_, App>) -> Option<String> {
    let rows = harness.get_all(By::new().role(Role::Row));
    rows.filter(|row| row.rect().top() > 90.0)
        .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .and_then(|row| row.accesskit_node().label())
}

#[test]
fn the_list_keeps_its_scroll_when_the_card_opens_and_closes() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window_speaking(
        backend,
        Vec2::new(1360.0, 700.0),
        Arc::new(|_| long_catalog()),
        Lang::Uk,
    );
    settle(&mut harness);
    let list = harness
        .get_by_role_and_label(Role::Row, "Програма 03")
        .rect()
        .center();
    harness.hover_at(list);
    for _ in 0..10 {
        harness.event(eframe::egui::Event::MouseWheel {
            unit: eframe::egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, -100.0),
            phase: eframe::egui::TouchPhase::Move,
            modifiers: eframe::egui::Modifiers::NONE,
            source: eframe::egui::MouseWheelSource::Wheel,
        });
        harness.step();
    }
    settle(&mut harness);
    let scrolled = top_row(&harness);
    assert_ne!(
        scrolled.as_deref(),
        Some("Програма 00"),
        "the wheel scrolls the list"
    );
    open_removal(&mut harness, "Програма 30");
    assert_eq!(top_row(&harness), scrolled, "opening the card keeps the scroll");
    confirm(&mut harness);
    settle(&mut harness);
    assert_eq!(top_row(&harness), scrolled, "closing the card keeps the scroll");
    harness.get_by_role_and_label(Role::Row, "Програма 33").click();
    settle(&mut harness);
    assert_eq!(top_row(&harness), scrolled, "the next click keeps the scroll");
}

/// Drags across a piece of text and copies what got selected.
fn select_and_copy(harness: &mut Harness<'_, App>, rect: eframe::egui::Rect) -> Option<String> {
    let start = rect.left_center() + Vec2::new(1.0, 0.0);
    let end = rect.right_center() - Vec2::new(1.0, 0.0);
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    harness.hover_at(end);
    harness.step();
    harness.drop_at(end);
    harness.step();
    harness.event(eframe::egui::Event::Copy);
    harness.step();
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            eframe::egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
}

#[test]
fn text_of_the_card_can_be_selected_and_copied() {
    let mut harness = window(FakeBackend::new(Outcome::Done));
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "Редактор").click();
    settle(&mut harness);
    let licence = harness.get_by_label("GPL-3.0").rect();
    assert_eq!(select_and_copy(&mut harness, licence).as_deref(), Some("GPL-3.0"));
    let summary = harness.get_by_label("Опис Редактор").rect();
    assert_eq!(
        select_and_copy(&mut harness, summary).as_deref(),
        Some("Опис Редактор")
    );
}

fn window_with_a_launcher(backend: Arc<FakeBackend>) -> Harness<'static, App> {
    let scanner: Scanner = Arc::new(|_| {
        let mut catalog = catalog();
        for item in &mut catalog.items {
            if item.package == "editor" {
                item.launch = Some(Launch::Desktop(PathBuf::from(
                    "/usr/share/applications/editor.desktop",
                )));
            }
        }
        catalog
    });
    window_speaking(backend, Vec2::new(1360.0, 1200.0), scanner, Lang::Uk)
}

#[test]
fn the_card_launches_the_program_through_its_launcher() {
    let backend = FakeBackend::new(Outcome::Done);
    let mut harness = window_with_a_launcher(Arc::clone(&backend));
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "Програвач").click();
    settle(&mut harness);
    assert!(
        harness.query_by_label("Запустити").is_none(),
        "a package without a launcher has nothing to launch"
    );
    harness.get_by_role_and_label(Role::Row, "Редактор").click();
    settle(&mut harness);
    harness.get_by_label("Запустити").click();
    settle(&mut harness);
    let argv: Vec<Vec<String>> = backend.commands().into_iter().map(|spec| spec.argv).collect();
    assert_eq!(
        argv,
        [["gio", "launch", "/usr/share/applications/editor.desktop"]]
    );
}

#[test]
fn a_failed_launch_says_so() {
    let backend = FakeBackend::new(Outcome::Failed(Failure::Code(1)));
    let mut harness = window_with_a_launcher(backend);
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Row, "Редактор").click();
    settle(&mut harness);
    harness.get_by_label("Запустити").click();
    let mut said = false;
    for _ in 0..40 {
        harness.step();
        sleep(Duration::from_millis(20));
        if harness
            .query_by_label_contains("Не вдалося запустити «Редактор»")
            .is_some()
        {
            said = true;
            break;
        }
    }
    assert!(said, "a toast says the launch failed");
}
