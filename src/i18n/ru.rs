//! The Russian catalogue.

use super::{DateStyle, Text};
use crate::model::{Section, SourceKind};

/// Picks the Russian form for a count: 1 день, 2 дня, 5 дней.
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
        Section::Internet => "Интернет",
        Section::Multimedia => "Мультимедиа",
        Section::Graphics => "Графика",
        Section::Office => "Офис",
        Section::Development => "Разработка",
        Section::Games => "Игры",
        Section::Education => "Образование",
        Section::Science => "Наука",
        Section::System => "Система",
        Section::Utilities => "Утилиты",
        Section::Console => "Консольные",
        Section::Libraries => "Библиотеки",
        Section::Fonts => "Шрифты",
        Section::Drivers => "Ядро и драйверы",
        Section::Meta => "Метапакеты",
    }
}

fn source(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Repo => "Репозитории",
        SourceKind::Aur => "AUR",
        SourceKind::Flatpak => "Flatpak",
        SourceKind::Steam => "Steam",
        SourceKind::Cargo => "Cargo",
        SourceKind::Node => "Node.js",
        SourceKind::Python => "Python",
        SourceKind::Rustup => "Rustup",
        SourceKind::LocalDeb => "Локальные пакеты",
        SourceKind::Docker => "Docker",
        SourceKind::Manual => "Вне менеджеров",
    }
}

fn packages(count: usize) -> String {
    format!("{count} {}", plural(count as i64, "пакет", "пакета", "пакетов"))
}

pub static TEXT: Text = Text {
    app_name: "Склад",
    language: "Язык",
    no_matches: "Ничего не найдено",

    reading_system: "Читаю систему…",
    nothing_found: "Ничего не найдено. Измените поиск или фильтры.",
    dark_theme: "Тёмная тема",
    refresh: "Обновить",
    search: "Поиск",
    clear_search: "Очистить поиск",
    removed: |title| format!("Удалено: {title}"),
    records: |count| format!("{count} {}", plural(count as i64, "запись", "записи", "записей")),

    sort_by: "Сортировать:",
    sort_last_run: "Последний запуск",
    sort_name: "Название",
    sort_size: "Размер",
    sort_installed: "Установлено",
    ascending: "По возрастанию",
    descending: "По убыванию",
    origin_explicit: "Установленные вами",
    origin_needed: "Нужные зависимости",
    origin_orphans: "Ненужные зависимости",

    heading_programs: "Программы",
    other: "Другое",
    heading_origin: "Происхождение",
    source_heading: "Источник",
    reset_filters: "Сбросить фильтры",
    remove_filter: |name| format!("Убрать {name}"),
    section,
    source,

    column_program: "Программа",
    column_size: "Размер",
    column_last_run: "Последний запуск",

    badge_orphan: "Ненужная зависимость",
    badge_running: "Работает",
    key_version: "Версия",
    key_package: "Пакет",
    key_license: "Лицензия",
    key_developer: "Разработчик",
    key_size: "Размер",
    key_installed: "Установлено",
    key_reason: "Как попал в систему",
    key_launch: "Последний запуск",
    key_groups: "Группы",
    key_removal: "Как удаляется",
    reason_explicit: "Вы установили его сами",
    reason_dependency: "Как зависимость другого пакета",
    heading_description: "Описание",
    heading_launchers: "Ярлыки",
    heading_command: "Команда удаления",
    needed_by_others: "Пакет нужен другим программам. Варианты показаны в окне удаления.",
    required_by: |count| format!("Нужен пакетам ({count})"),
    depends_on: |count| format!("Зависит от ({count})"),
    and_more: |count| format!("и ещё {count}"),
    remove: "Удалить",
    launch: "Запустить",
    launch_failed: |title| format!("Не удалось запустить «{title}»"),
    site: "Сайт",
    copy: "Скопировать",
    by_package: |manager| format!("Через менеджер пакетов {manager}"),
    by_steam: "Через клиент Steam",
    by_language_manager: "Через менеджер пакетов своего языка",
    by_flatpak: "Через Flatpak",
    by_cargo: "Через cargo",
    not_removable_here: "Отсюда удалить нельзя",
    files_to_bin: "Файлы перемещаются в корзину",
    files_for_good: "Файлы удаляются навсегда, нужен пароль",

    size_units: ["Б", "КБ", "МБ", "ГБ", "ТБ"],
    decimal_mark: ',',
    date_style: DateStyle::DayFirst,
    today: "сегодня",
    yesterday: "вчера",
    days_ago: |days| format!("{days} {} назад", plural(days, "день", "дня", "дней")),
    months_ago: |months| format!("{months} {} назад", plural(months, "месяц", "месяца", "месяцев")),
    years_ago: |years| format!("{years} {} назад", plural(years, "год", "года", "лет")),
    running_now: "работает сейчас",
    no_record: "нет записи",
    running_now_long: "Работает сейчас",
    no_record_long: "Нет записи. Plasma видит только запуски через меню, панель и KRunner.",
    not_tracked_long: "Запуски записей такого типа не отслеживаются",
    plasma_log: "журнал Plasma",
    last_launch: |when, source| format!("{when}, по данным: {source}"),

    confirm_title: |title| format!("Удалить «{title}»?"),
    other_list_refused: |manager| {
        format!(
            "{manager} удалил бы другой список пакетов, поэтому действие остановлено. Обновите каталог и попробуйте ещё раз."
        )
    },
    manager_refused: |manager, message| format!("{manager} отказал:\n{message}"),
    path_too_close: "Этот путь слишком близко к корню системы, поэтому удаление запрещено.",
    no_password: "Пароль не введён, ничего не изменено.",
    needed_by_programs: |names| format!("Этот пакет нужен программам: {names}"),
    keep_everything: "Оставить всё как есть",
    remove_with_them: "Удалить вместе с ними",
    held_by_meta: |metas, kept| {
        format!(
            "Программу держит метапакет {metas}. Он тоже будет удалён, но это лишь пустая обёртка. Остальные его пакеты останутся в системе и будут отмечены как установленные вами: {kept}."
        )
    },
    critical_packages: |names| format!("Без этих пакетов система может не запуститься: {names}"),
    plan_summary: |count, freed| {
        format!(
            "Будет удалено {}, освободится {freed}. Настройки в домашней папке останутся.",
            packages(count)
        )
    },
    through_manager: |name| format!("Будет удалено через {name}."),
    flatpak_note: "Программа будет удалена через Flatpak. Её данные в домашней папке останутся.",
    steam_note: "Откроется окно Steam. Подтвердите удаление там, а затем обновите каталог.",
    cargo_note: "Программа будет удалена через cargo.",
    files_go_to_bin: "Файлы будут перемещены в корзину:",
    files_go_for_good: "Файлы будут удалены безвозвратно:",
    not_a_package: "Это не пакет: вернуть файлы можно будет только из снимка или резервной копии.",
    checking: |manager| format!("Проверяю вместе с {manager}…"),
    removing: "Удаляю…",
    done: "Готово",
    failed: |message| format!("Не удалось удалить: {message}. Посмотрите вывод выше."),
    close: "Закрыть",
    cancel: "Отмена",
    copy_command: "Скопировать команду",
    still_running: "Программа сейчас работает. Закройте её перед удалением.",
    understood: "Я понимаю последствия",
    exit_code: |code| format!("код завершения {code}"),
    interrupted: "процесс прерван",
    empty_command: "пустая команда",

    steam_component: "Компонент Steam",
    steam_game: "Игра из Steam",
    programs: |names| format!("Программы: {names}"),
    loose_file: |path| format!("Файл вне менеджеров пакетов: {path}"),
    loose_launcher: |path| format!("Ярлык вне менеджеров пакетов: {path}"),
    stray_python: |path| {
        format!(
            "Запись лежит в системном Python вне менеджера пакетов: {path}. Это может быть остаток после обновления, поэтому автоматическое удаление отключено."
        )
    },
    python_tool: |manager| format!("Инструмент Python в отдельном окружении, установлен через {manager}"),
    python_interpreter: |manager| format!("Интерпретатор Python, установленный через {manager}"),
    python_environment: |tool| format!("Окружение Python от {tool}"),
    by_docker: "Через Docker",
    docker_container: |image| format!("Контейнер Docker из образа {image}"),
    docker_image: "Образ Docker",
    docker_volume: "Том Docker: данные, которые хранят контейнеры",
    docker_data_lost: "Его содержимое не вернуть: Docker удаляет его навсегда.",
    docker_project: |count| {
        let containers = plural(count as i64, "контейнер", "контейнера", "контейнеров");
        format!("Проект Docker Compose: {count} {containers}")
    },
    heading_docker_parts: "Что входит",
    docker_part_container: "Контейнер",
    docker_part_image: "Образ",
    docker_part_volume: "Том",
    rust_toolchain: "Инструментарий Rust: компилятор, cargo и стандартная библиотека",
    default_toolchain: "Это инструментарий по умолчанию: без него cargo и rustc перестанут работать.",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_forms_follow_russian_rules() {
        let form = |n| plural(n, "год", "года", "лет");
        assert_eq!(
            [form(1), form(3), form(5), form(12), form(21), form(104)],
            ["год", "года", "лет", "лет", "год", "года"]
        );
    }
}
