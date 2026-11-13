//! The Ukrainian catalogue.

use super::{DateStyle, Text};
use crate::model::{Section, SourceKind};

/// Picks the Ukrainian form for a count: 1 день, 2 дні, 5 днів.
fn plural<'a>(count: i64, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let tens = count % 100;
    let units = count % 10;
    if (11..=14).contains(&tens) {
        many
    } else if units == 1 {
        one
    } else if (2..=4).contains(&units) {
        few
    } else {
        many
    }
}

fn section(section: Section) -> &'static str {
    match section {
        Section::Internet => "Інтернет",
        Section::Multimedia => "Мультимедіа",
        Section::Graphics => "Графіка",
        Section::Office => "Офіс",
        Section::Development => "Розробка",
        Section::Games => "Ігри",
        Section::Education => "Освіта",
        Section::Science => "Наука",
        Section::System => "Система",
        Section::Utilities => "Утиліти",
        Section::Console => "Консольні",
        Section::Libraries => "Бібліотеки",
        Section::Fonts => "Шрифти",
        Section::Drivers => "Ядро і драйвери",
        Section::Meta => "Метапакети",
    }
}

fn source(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Repo => "Репозиторії",
        SourceKind::Aur => "AUR",
        SourceKind::Flatpak => "Flatpak",
        SourceKind::Steam => "Steam",
        SourceKind::Cargo => "Cargo",
        SourceKind::Node => "Node.js",
        SourceKind::Python => "Python",
        SourceKind::Rustup => "Rustup",
        SourceKind::LocalDeb => "Локальні пакети",
        SourceKind::Docker => "Docker",
        SourceKind::Manual => "Поза менеджерами",
    }
}

fn packages(count: usize) -> String {
    format!("{count} {}", plural(count as i64, "пакет", "пакети", "пакетів"))
}

pub static TEXT: Text = Text {
    app_name: "Склад",
    language: "Мова",
    no_matches: "Нічого не знайдено",

    reading_system: "Читаю систему…",
    nothing_found: "Нічого не знайдено. Змініть пошук або фільтри.",
    dark_theme: "Темна тема",
    refresh: "Оновити",
    search: "Пошук",
    clear_search: "Очистити пошук",
    removed: |title| format!("Видалено: {title}"),
    records: |count| format!("{count} {}", plural(count as i64, "запис", "записи", "записів")),
    shown_of: |shown, total| {
        let noun = if total % 10 == 1 && total % 100 != 11 {
            "запису"
        } else {
            "записів"
        };
        format!("{shown} з {total} {noun}")
    },
    filters: "Фільтри",

    sort_by: "Сортувати:",
    sort_last_run: "Останній запуск",
    sort_name: "Назва",
    sort_size: "Розмір",
    sort_installed: "Встановлено",
    ascending: "За зростанням",
    descending: "За спаданням",
    origin_explicit: "Встановлені вами",
    origin_needed: "Потрібні залежності",
    origin_orphans: "Непотрібні залежності",

    heading_programs: "Програми",
    other: "Інше",
    heading_origin: "Походження",
    source_heading: "Джерело",
    reset_filters: "Скинути фільтри",
    remove_filter: |name| format!("Прибрати {name}"),
    section,
    source,

    column_program: "Програма",
    column_size: "Розмір",
    column_last_run: "Останній запуск",

    badge_orphan: "Непотрібна залежність",
    badge_running: "Працює",
    key_version: "Версія",
    key_package: "Пакет",
    key_license: "Ліцензія",
    key_developer: "Розробник",
    key_size: "Розмір",
    key_installed: "Встановлено",
    key_reason: "Як потрапив у систему",
    key_launch: "Останній запуск",
    key_groups: "Групи",
    key_removal: "Як видаляється",
    reason_explicit: "Ви встановили його самі",
    reason_dependency: "Як залежність іншого пакета",
    heading_description: "Опис",
    heading_launchers: "Ярлики",
    heading_command: "Команда видалення",
    needed_by_others: "Пакет потрібен іншим програмам. Варіанти показані у вікні видалення.",
    required_by: |count| format!("Потрібен пакетам ({count})"),
    depends_on: |count| format!("Залежить від ({count})"),
    and_more: |count| format!("і ще {count}"),
    remove: "Видалити",
    launch: "Запустити",
    launch_failed: |title| format!("Не вдалося запустити «{title}»"),
    site: "Сайт",
    copy: "Скопіювати",
    by_package: |manager| format!("Через менеджер пакетів {manager}"),
    by_steam: "Через клієнт Steam",
    by_language_manager: "Через менеджер пакетів своєї мови",
    by_flatpak: "Через Flatpak",
    by_cargo: "Через cargo",
    not_removable_here: "Звідси видалити не можна",
    files_to_bin: "Файли переміщуються в кошик",
    files_for_good: "Файли видаляються назавжди, потрібен пароль",

    size_units: ["Б", "КБ", "МБ", "ГБ", "ТБ"],
    decimal_mark: ',',
    date_style: DateStyle::DayFirst,
    today: "сьогодні",
    yesterday: "вчора",
    days_ago: |days| format!("{days} {} тому", plural(days, "день", "дні", "днів")),
    months_ago: |months| format!("{months} {} тому", plural(months, "місяць", "місяці", "місяців")),
    years_ago: |years| format!("{years} {} тому", plural(years, "рік", "роки", "років")),
    running_now: "працює зараз",
    no_record: "немає запису",
    running_now_long: "Працює зараз",
    no_record_long: "Немає запису. Plasma бачить лише запуски через меню, панель і KRunner.",
    not_tracked_long: "Запуски такого типу записів не відстежуються",
    plasma_log: "журнал Plasma",
    last_launch: |when, source| format!("{when}, за даними: {source}"),

    confirm_title: |title| format!("Видалити «{title}»?"),
    other_list_refused: |manager| {
        format!(
            "{manager} видалив би інший список пакетів, тому дію зупинено. Оновіть каталог і спробуйте ще раз."
        )
    },
    manager_refused: |manager, message| format!("{manager} відмовив:\n{message}"),
    path_too_close: "Цей шлях надто близько до кореня системи, тому видалення заборонено.",
    no_password: "Пароль не введено, нічого не змінено.",
    needed_by_programs: |names| format!("Цей пакет потрібен програмам: {names}"),
    keep_everything: "Лишити все як є",
    remove_with_them: "Видалити разом із ними",
    held_by_meta: |metas, kept| {
        format!(
            "Програму тримає метапакет {metas}. Він теж буде видалений, але це лише порожня обгортка. Решта його пакетів лишиться в системі й буде позначена як встановлена вами: {kept}."
        )
    },
    critical_packages: |names| format!("Без цих пакетів система може не запуститися: {names}"),
    plan_summary: |count, freed| {
        format!(
            "Буде видалено {}, звільниться {freed}. Налаштування в домашній теці лишаться.",
            packages(count)
        )
    },
    through_manager: |name| format!("Буде видалено через {name}."),
    flatpak_note: "Програму буде видалено через Flatpak. Її дані в домашній теці лишаться.",
    steam_note: "Відкриється вікно Steam. Підтвердьте видалення там, а потім оновіть каталог.",
    cargo_note: "Програму буде видалено через cargo.",
    files_go_to_bin: "Файли буде переміщено в кошик:",
    files_go_for_good: "Файли буде видалено безповоротно:",
    not_a_package: "Це не пакет: повернути файли можна буде лише зі знімка або резервної копії.",
    checking: |manager| format!("Перевіряю разом із {manager}…"),
    removing: "Видаляю…",
    done: "Готово",
    failed: |message| format!("Не вдалося видалити: {message}. Подивіться вивід вище."),
    close: "Закрити",
    cancel: "Скасувати",
    copy_command: "Скопіювати команду",
    still_running: "Програма зараз працює. Закрийте її перед видаленням.",
    understood: "Я розумію наслідки",
    exit_code: |code| format!("код завершення {code}"),
    interrupted: "процес перервано",
    empty_command: "порожня команда",

    steam_component: "Компонент Steam",
    steam_game: "Гра зі Steam",
    programs: |names| format!("Програми: {names}"),
    loose_file: |path| format!("Файл поза менеджерами пакетів: {path}"),
    loose_launcher: |path| format!("Ярлик поза менеджерами пакетів: {path}"),
    stray_python: |path| {
        format!(
            "Запис лежить у системному Python поза менеджером пакетів: {path}. Це може бути залишок після оновлення, тому автоматичне видалення вимкнено."
        )
    },
    python_tool: |manager| format!("Інструмент Python в окремому середовищі, встановлено через {manager}"),
    python_interpreter: |manager| format!("Інтерпретатор Python, встановлений через {manager}"),
    python_environment: |tool| format!("Середовище Python від {tool}"),
    by_docker: "Через Docker",
    docker_container: |image| format!("Контейнер Docker з образу {image}"),
    docker_image: "Образ Docker",
    docker_volume: "Том Docker: дані, які зберігають контейнери",
    docker_data_lost: "Його вміст не повернути: Docker видаляє його назавжди.",
    docker_project: |count| {
        let containers = plural(count as i64, "контейнер", "контейнери", "контейнерів");
        format!("Проєкт Docker Compose: {count} {containers}")
    },
    heading_docker_parts: "Що входить",
    docker_part_container: "Контейнер",
    docker_part_image: "Образ",
    docker_part_volume: "Том",
    rust_toolchain: "Інструментарій Rust: компілятор, cargo і стандартна бібліотека",
    default_toolchain: "Це типовий інструментарій: без нього cargo і rustc перестануть працювати.",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_forms_follow_ukrainian_rules() {
        let form = |n| plural(n, "день", "дні", "днів");
        assert_eq!(
            [form(1), form(2), form(5), form(11), form(21), form(112)],
            ["день", "дні", "днів", "днів", "день", "днів"]
        );
    }
}
