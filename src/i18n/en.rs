//! The English catalogue.

use super::{DateStyle, Text};
use crate::model::{Section, SourceKind};

fn counted(count: i64, noun: &str) -> String {
    let ending = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{ending}")
}

fn section(section: Section) -> &'static str {
    match section {
        Section::Internet => "Internet",
        Section::Multimedia => "Multimedia",
        Section::Graphics => "Graphics",
        Section::Office => "Office",
        Section::Development => "Development",
        Section::Games => "Games",
        Section::Education => "Education",
        Section::Science => "Science",
        Section::System => "System",
        Section::Utilities => "Utilities",
        Section::Console => "Console",
        Section::Libraries => "Libraries",
        Section::Fonts => "Fonts",
        Section::Drivers => "Kernel and drivers",
        Section::Meta => "Metapackages",
    }
}

fn source(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Repo => "Repositories",
        SourceKind::Aur => "AUR",
        SourceKind::Flatpak => "Flatpak",
        SourceKind::Steam => "Steam",
        SourceKind::Cargo => "Cargo",
        SourceKind::Node => "Node.js",
        SourceKind::Python => "Python",
        SourceKind::Rustup => "Rustup",
        SourceKind::LocalDeb => "Local packages",
        SourceKind::Docker => "Docker",
        SourceKind::Manual => "Outside managers",
    }
}

pub static TEXT: Text = Text {
    app_name: "Sklad",
    language: "Language",
    no_matches: "No matches",

    reading_system: "Reading the system…",
    nothing_found: "Nothing found. Change the search or the filters.",
    dark_theme: "Dark theme",
    refresh: "Refresh",
    search: "Search",
    clear_search: "Clear search",
    removed: |title| format!("Removed: {title}"),
    records: |count| match count {
        1 => "1 entry".to_owned(),
        _ => format!("{count} entries"),
    },
    shown_of: |shown, total| match total {
        1 => format!("{shown} of 1 entry"),
        _ => format!("{shown} of {total} entries"),
    },
    filters: "Filters",

    sort_by: "Sort by:",
    sort_last_run: "Last launch",
    sort_name: "Name",
    sort_size: "Size",
    sort_installed: "Installed",
    ascending: "Ascending",
    descending: "Descending",
    origin_explicit: "Installed by you",
    origin_needed: "Needed dependencies",
    origin_orphans: "Unneeded dependencies",

    heading_programs: "Programs",
    other: "Other",
    heading_origin: "Origin",
    source_heading: "Source",
    reset_filters: "Reset filters",
    remove_filter: |name| format!("Remove {name}"),
    section,
    source,

    column_program: "Program",
    column_size: "Size",
    column_last_run: "Last launch",

    badge_orphan: "Unneeded dependency",
    badge_running: "Running",
    key_version: "Version",
    key_package: "Package",
    key_license: "License",
    key_developer: "Developer",
    key_size: "Size",
    key_installed: "Installed",
    key_reason: "How it got here",
    key_launch: "Last launch",
    key_groups: "Groups",
    key_removal: "How it is removed",
    reason_explicit: "You installed it",
    reason_dependency: "As a dependency of another package",
    heading_description: "Description",
    heading_launchers: "Launchers",
    heading_command: "Removal command",
    needed_by_others: "Other programs need this package. The options are in the removal window.",
    required_by: |count| format!("Required by ({count})"),
    depends_on: |count| format!("Depends on ({count})"),
    and_more: |count| format!("and {count} more"),
    remove: "Remove",
    launch: "Launch",
    launch_failed: |title| format!("Could not launch \"{title}\""),
    site: "Website",
    copy: "Copy",
    by_package: |manager| format!("Through the {manager} package manager"),
    by_steam: "Through the Steam client",
    by_language_manager: "Through the package manager of its language",
    by_flatpak: "Through Flatpak",
    by_cargo: "Through cargo",
    not_removable_here: "Cannot be removed from here",
    files_to_bin: "The files are moved to the bin",
    files_for_good: "The files are deleted for good, a password is needed",

    size_units: ["B", "KB", "MB", "GB", "TB"],
    decimal_mark: '.',
    date_style: DateStyle::YearFirst,
    today: "today",
    yesterday: "yesterday",
    days_ago: |days| format!("{} ago", counted(days, "day")),
    months_ago: |months| format!("{} ago", counted(months, "month")),
    years_ago: |years| format!("{} ago", counted(years, "year")),
    running_now: "running now",
    no_record: "no record",
    running_now_long: "Running now",
    no_record_long: "No record. Plasma only sees launches from the menu, the panel and KRunner.",
    not_tracked_long: "Launches of this kind of entry are not tracked",
    plasma_log: "the Plasma log",
    last_launch: |when, source| format!("{when}, according to {source}"),

    confirm_title: |title| format!("Remove \"{title}\"?"),
    other_list_refused: |manager| {
        format!(
            "{manager} would remove a different list of packages, so nothing was done. Refresh the catalogue and try again."
        )
    },
    manager_refused: |manager, message| format!("{manager} refused:\n{message}"),
    path_too_close: "This path is too close to the root of the system, so removal is forbidden.",
    no_password: "No password was entered, nothing was changed.",
    needed_by_programs: |names| format!("These programs need this package: {names}"),
    keep_everything: "Keep everything as it is",
    remove_with_them: "Remove together with them",
    held_by_meta: |metas, kept| {
        format!(
            "The metapackage {metas} holds this program. It will be removed too, but it is only an empty wrapper. The rest of its packages stay in the system and are marked as installed by you: {kept}."
        )
    },
    critical_packages: |names| format!("The system may not start without these packages: {names}"),
    plan_summary: |count, freed| {
        format!(
            "{} will be removed, {freed} will be freed. Settings in the home folder stay.",
            counted(count as i64, "package")
        )
    },
    through_manager: |name| format!("Will be removed through {name}."),
    flatpak_note: "The program will be removed through Flatpak. Its data in the home folder stays.",
    steam_note: "A Steam window will open. Confirm the removal there, then refresh the catalogue.",
    cargo_note: "The program will be removed through cargo.",
    files_go_to_bin: "The files will be moved to the bin:",
    files_go_for_good: "The files will be deleted for good:",
    not_a_package: "This is not a package: the files can only be restored from a snapshot or a backup.",
    checking: |manager| format!("Checking with {manager}…"),
    removing: "Removing…",
    done: "Done",
    failed: |message| format!("Could not remove: {message}. See the output above."),
    close: "Close",
    cancel: "Cancel",
    copy_command: "Copy command",
    still_running: "The program is running. Close it before removing.",
    understood: "I understand the consequences",
    exit_code: |code| format!("exit code {code}"),
    interrupted: "the process was interrupted",
    empty_command: "empty command",

    steam_component: "Steam component",
    steam_game: "Steam game",
    programs: |names| format!("Programs: {names}"),
    loose_file: |path| format!("A file outside package managers: {path}"),
    loose_launcher: |path| format!("A launcher outside package managers: {path}"),
    stray_python: |path| {
        format!(
            "This record lies in the system Python outside the package manager: {path}. It may be left over from an update, so automatic removal is off."
        )
    },
    python_tool: |manager| format!("A Python tool in its own environment, installed through {manager}"),
    python_interpreter: |manager| format!("A Python interpreter installed through {manager}"),
    python_environment: |tool| format!("A Python environment of {tool}"),
    by_docker: "Through Docker",
    docker_container: |image| format!("A Docker container made from {image}"),
    docker_image: "A Docker image",
    docker_volume: "A Docker volume: data kept for containers",
    docker_data_lost: "What it holds cannot be brought back: Docker deletes it for good.",
    docker_project: |count| {
        let containers = if count == 1 { "container" } else { "containers" };
        format!("A Docker Compose project: {count} {containers}")
    },
    heading_docker_parts: "What it holds",
    docker_part_container: "Container",
    docker_part_image: "Image",
    docker_part_volume: "Volume",
    rust_toolchain: "A Rust toolchain: the compiler, cargo and the standard library",
    default_toolchain: "This is the default toolchain: cargo and rustc stop working without it.",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_counts_read_naturally() {
        assert_eq!((TEXT.days_ago)(1), "1 day ago");
        assert_eq!((TEXT.days_ago)(5), "5 days ago");
        assert_eq!((TEXT.records)(1), "1 entry");
        assert_eq!((TEXT.records)(2), "2 entries");
        assert_eq!(
            (TEXT.plan_summary)(1, "1.0 MB"),
            "1 package will be removed, 1.0 MB will be freed. Settings in the home folder stay."
        );
    }
}
