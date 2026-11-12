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
        SourceKind::Manual => "Вручну",
    }
}

fn packages(count: usize) -> String {
    format!("{count} {}", plural(count as i64, "пакет", "пакети", "пакетів"))
}

pub static TEXT: Text = Text {
    app_name: "Склад",
    switch_language: "English",
    switch_mark: "EN",

    reading_system: "Читаю систему…",
    nothing_found: "Нічого не знайдено. Змініть пошук або фільтри.",
    dark_theme: "Темна тема",
    refresh: "Оновити",
    search: "Пошук",
    clear_search: "Очистити пошук",
    removed: |title| format!("Видалено: {title}"),
    records: |count| format!("{count} {}", plural(count as i64, "запис", "записи", "записів")),

    sort_by: "Сортувати:",
    sort_last_run: "Запуск",
    sort_name: "Назва",
    sort_size: "Розмір",
    sort_installed: "Встановлено",
    ascending: "За зростанням",
    descending: "За спаданням",
    origin_all: "Усі",
    origin_explicit: "Вручну",
    origin_dependency: "Залежності",
    origin_orphans: "Сироти",

    all_programs: "Усі програми",
    other: "Інше",
    everything: "Геть усе",
    source_heading: "Джерело",
    all_sources: "Усі джерела",
    section,
    source,

    column_program: "Програма",
    column_size: "Розмір",
    column_last_run: "Останній запуск",

    badge_orphan: "Сирота",
    badge_running: "Працює",
    key_version: "Версія",
    key_package: "Пакет",
    key_license: "Ліцензія",
    key_developer: "Розробник",
    key_size: "Розмір",
    key_installed: "Встановлено",
    key_reason: "Причина",
    key_launch: "Запуск",
    key_groups: "Групи",
    key_removal: "Видалення",
    reason_explicit: "Вручну",
    reason_dependency: "Як залежність",
    heading_description: "Опис",
    heading_launchers: "Ярлики",
    heading_command: "Команда видалення",
    needed_by_others: "Пакет потрібен іншим програмам. Варіанти показані у вікні видалення.",
    required_by: |count| format!("Потрібен для ({count})"),
    depends_on: |count| format!("Залежить від ({count})"),
    and_more: |count| format!("і ще {count}"),
    remove: "Видалити",
    launch: "Запустити",
    launch_failed: |title| format!("Не вдалося запустити «{title}»"),
    site: "Сайт",
    copy: "Скопіювати",
    by_pacman: "Менеджер пакетів pacman",
    by_steam: "Клієнт Steam",
    by_language_manager: "Менеджер пакетів мови",
    not_removable_here: "Недоступне з цієї програми",
    files_to_bin: "Файли в кошик",
    files_for_good: "Файли безповоротно, потрібен пароль",

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
    not_tracked_long: "Для такого типу не відстежується",
    plasma_log: "журнал Plasma",
    last_launch: |when, source| format!("{when}, джерело: {source}"),

    confirm_title: |title| format!("Видалити «{title}»?"),
    other_list_refused: "pacman видалив би інший список пакетів, тому дію зупинено. Оновіть каталог і спробуйте ще раз.",
    pacman_refused: |message| format!("pacman відмовив:\n{message}"),
    path_too_close: "Цей шлях надто близько до кореня системи, тому видалення заборонено.",
    no_password: "Пароль не введено, нічого не змінено.",
    needed_by_programs: |names| format!("Цей пакет потрібен програмам: {names}"),
    keep_everything: "Лишити все як є",
    remove_with_them: "Видалити разом із ними",
    held_by_meta: |metas, kept| {
        format!(
            "Програму тримає метапакет {metas}. Він теж буде видалений, але це лише порожня обгортка. Решта його пакетів лишиться в системі. Позначено як встановлені вручну: {kept}."
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
    checking: "Перевіряю разом із pacman…",
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
    uv_python: "Інтерпретатор Python, встановлений через uv",
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
