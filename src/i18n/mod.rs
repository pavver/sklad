//! Every word of the interface. `Text` is one record per language, so a string missing from a
//! language is a build error, not an empty place on screen.

mod en;
mod fr;
mod ru;
mod uk;

use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Section, Source, SourceKind};
use crate::remove::Failure;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Uk,
    En,
    Fr,
    Ru,
}

impl Lang {
    /// The tag used by desktop files, AppStream and the saved choice.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Uk => "uk",
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Ru => "ru",
        }
    }

    pub fn text(self) -> &'static Text {
        match self {
            Lang::Uk => &uk::TEXT,
            Lang::En => &en::TEXT,
            Lang::Fr => &fr::TEXT,
            Lang::Ru => &ru::TEXT,
        }
    }

    /// Every language with a catalogue, in the order of the language choice.
    pub const ALL: [Lang; 4] = [Lang::Uk, Lang::En, Lang::Fr, Lang::Ru];

    /// The name in the language itself, then in English, so a search finds it either way.
    /// English needs no second name.
    pub fn name(self) -> &'static str {
        match self {
            Lang::Uk => "Українська (Ukrainian)",
            Lang::En => "English",
            Lang::Fr => "Français (French)",
            Lang::Ru => "Русский (Russian)",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code.trim() {
            "uk" => Some(Lang::Uk),
            "en" => Some(Lang::En),
            "fr" => Some(Lang::Fr),
            "ru" => Some(Lang::Ru),
            _ => None,
        }
    }

    /// The language of a locale value such as `uk_UA.UTF-8`, or of a `LANGUAGE` list such as
    /// `uk:en_US`, whose first entry is the preferred one. Anything without a catalogue is English.
    pub fn of_locale(value: &str) -> Self {
        let first = value.split(':').next().unwrap_or_default();
        let code = first.split(['_', '.', '@']).next().unwrap_or_default();
        Self::from_code(code).unwrap_or(Lang::En)
    }

    /// The language of the session, read in gettext's order of precedence.
    pub fn of_session() -> Self {
        let chosen = ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(std::env::var_os)
            .map(|value| value.to_string_lossy().into_owned())
            .find(|value| !value.is_empty());
        chosen.map_or(Lang::En, |value| Self::of_locale(&value))
    }
}

/// The language in use and the file that remembers it. Tests run without a file.
#[derive(Clone, Debug)]
pub struct Choice {
    pub lang: Lang,
    pub file: Option<PathBuf>,
}

impl Choice {
    /// A language for this run only.
    pub fn fixed(lang: Lang) -> Self {
        Self { lang, file: None }
    }

    /// The saved language, or the one of the session when nothing was chosen yet.
    pub fn load(file: PathBuf) -> Self {
        let saved = fs::read_to_string(&file).ok();
        let lang = saved.as_deref().and_then(Lang::from_code);
        Self {
            lang: lang.unwrap_or_else(Lang::of_session),
            file: Some(file),
        }
    }

    /// Where the choice lives: `$XDG_CONFIG_HOME/sklad/language`, or under `~/.config`.
    pub fn default_file(home: &Path) -> PathBuf {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map_or_else(|| home.join(".config"), PathBuf::from);
        config.join("sklad/language")
    }

    /// Switches the language. A file that cannot be written costs the memory of the choice, not the
    /// switch itself.
    pub fn set(&mut self, lang: Lang) {
        self.lang = lang;
        if let Some(file) = &self.file {
            if let Some(folder) = file.parent() {
                let _ = fs::create_dir_all(folder);
            }
            let _ = fs::write(file, lang.code());
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateStyle {
    /// `29.02.2024`
    DayFirst,
    /// `2024-02-29`
    YearFirst,
}

pub struct Text {
    pub app_name: &'static str,
    /// The name of the language choice.
    pub language: &'static str,
    /// Said by a list whose search found nothing.
    pub no_matches: &'static str,

    pub reading_system: &'static str,
    pub nothing_found: &'static str,
    pub dark_theme: &'static str,
    pub refresh: &'static str,
    pub search: &'static str,
    pub clear_search: &'static str,
    pub removed: fn(&str) -> String,
    pub records: fn(usize) -> String,

    pub sort_by: &'static str,
    pub sort_last_run: &'static str,
    pub sort_name: &'static str,
    pub sort_size: &'static str,
    pub sort_installed: &'static str,
    pub ascending: &'static str,
    pub descending: &'static str,
    pub origin_all: &'static str,
    pub origin_explicit: &'static str,
    pub origin_dependency: &'static str,
    pub origin_orphans: &'static str,

    pub all_programs: &'static str,
    pub other: &'static str,
    pub everything: &'static str,
    pub source_heading: &'static str,
    pub all_sources: &'static str,
    pub section: fn(Section) -> &'static str,
    pub source: fn(SourceKind) -> &'static str,

    pub column_program: &'static str,
    pub column_size: &'static str,
    pub column_last_run: &'static str,

    pub badge_orphan: &'static str,
    pub badge_running: &'static str,
    pub key_version: &'static str,
    pub key_package: &'static str,
    pub key_license: &'static str,
    pub key_developer: &'static str,
    pub key_size: &'static str,
    pub key_installed: &'static str,
    pub key_reason: &'static str,
    pub key_launch: &'static str,
    pub key_groups: &'static str,
    pub key_removal: &'static str,
    pub reason_explicit: &'static str,
    pub reason_dependency: &'static str,
    pub heading_description: &'static str,
    pub heading_launchers: &'static str,
    pub heading_command: &'static str,
    pub needed_by_others: &'static str,
    pub required_by: fn(usize) -> String,
    pub depends_on: fn(usize) -> String,
    pub and_more: fn(usize) -> String,
    pub remove: &'static str,
    pub launch: &'static str,
    pub launch_failed: fn(&str) -> String,
    pub site: &'static str,
    pub copy: &'static str,
    /// Names the package manager of the distribution.
    pub by_package: fn(&str) -> String,
    pub by_steam: &'static str,
    pub by_language_manager: &'static str,
    pub by_flatpak: &'static str,
    pub by_cargo: &'static str,
    pub not_removable_here: &'static str,
    pub files_to_bin: &'static str,
    pub files_for_good: &'static str,

    pub size_units: [&'static str; 5],
    pub decimal_mark: char,
    pub date_style: DateStyle,
    pub today: &'static str,
    pub yesterday: &'static str,
    pub days_ago: fn(i64) -> String,
    pub months_ago: fn(i64) -> String,
    pub years_ago: fn(i64) -> String,
    pub running_now: &'static str,
    pub no_record: &'static str,
    pub running_now_long: &'static str,
    pub no_record_long: &'static str,
    pub not_tracked_long: &'static str,
    pub plasma_log: &'static str,
    pub last_launch: fn(&str, &str) -> String,

    pub confirm_title: fn(&str) -> String,
    pub other_list_refused: fn(&str) -> String,
    /// The package manager, then what it said.
    pub manager_refused: fn(&str, &str) -> String,
    pub path_too_close: &'static str,
    pub no_password: &'static str,
    pub needed_by_programs: fn(&str) -> String,
    pub keep_everything: &'static str,
    pub remove_with_them: &'static str,
    pub held_by_meta: fn(&str, usize) -> String,
    pub critical_packages: fn(&str) -> String,
    pub plan_summary: fn(usize, &str) -> String,
    pub through_manager: fn(&str) -> String,
    pub flatpak_note: &'static str,
    pub steam_note: &'static str,
    pub cargo_note: &'static str,
    pub files_go_to_bin: &'static str,
    pub files_go_for_good: &'static str,
    pub not_a_package: &'static str,
    pub checking: fn(&str) -> String,
    pub removing: &'static str,
    pub done: &'static str,
    pub failed: fn(&str) -> String,
    pub close: &'static str,
    pub cancel: &'static str,
    pub copy_command: &'static str,
    pub still_running: &'static str,
    pub understood: &'static str,
    pub exit_code: fn(i32) -> String,
    pub interrupted: &'static str,
    pub empty_command: &'static str,

    pub steam_component: &'static str,
    pub steam_game: &'static str,
    pub programs: fn(&str) -> String,
    pub loose_file: fn(&str) -> String,
    pub loose_launcher: fn(&str) -> String,
    pub stray_python: fn(&str) -> String,
    pub python_tool: fn(&str) -> String,
    pub uv_python: &'static str,
    pub rust_toolchain: &'static str,
    pub default_toolchain: &'static str,
}

impl Text {
    /// The name of a source as a badge says it: a repository goes by its own name.
    pub fn source_label(&self, source: &Source) -> String {
        match source {
            Source::Repo(name) => name.clone(),
            other => (self.source)(other.kind()).to_owned(),
        }
    }

    pub fn failure(&self, failure: &Failure) -> String {
        match failure {
            Failure::Code(code) => (self.exit_code)(*code),
            Failure::Interrupted => self.interrupted.to_owned(),
            Failure::EmptyCommand => self.empty_command.to_owned(),
            Failure::NotStarted(message) => message.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_locale_gives_its_language_and_an_unknown_one_gives_english() {
        assert_eq!(Lang::of_locale("uk_UA.UTF-8"), Lang::Uk);
        assert_eq!(Lang::of_locale("uk:en_US"), Lang::Uk);
        assert_eq!(Lang::of_locale("en_GB.UTF-8"), Lang::En);
        assert_eq!(Lang::of_locale("fr_CA.UTF-8"), Lang::Fr);
        assert_eq!(Lang::of_locale("ru_RU.UTF-8"), Lang::Ru);
        assert_eq!(Lang::of_locale("de_DE.UTF-8"), Lang::En);
        assert_eq!(Lang::of_locale("C"), Lang::En);
        assert_eq!(Lang::of_locale(""), Lang::En);
    }

    #[test]
    fn a_choice_survives_a_restart() -> std::io::Result<()> {
        let folder = std::env::temp_dir().join(format!("sklad-choice-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let file = folder.join("sklad/language");
        let mut choice = Choice::load(file.clone());
        choice.set(Lang::En);
        assert_eq!(Choice::load(file.clone()).lang, Lang::En);
        choice.set(Lang::Uk);
        assert_eq!(Choice::load(file).lang, Lang::Uk);
        fs::remove_dir_all(&folder)
    }

    #[test]
    fn a_damaged_file_is_treated_as_no_choice() -> std::io::Result<()> {
        let file = std::env::temp_dir().join(format!("sklad-damaged-{}", std::process::id()));
        fs::write(&file, "klingon")?;
        assert_eq!(Choice::load(file.clone()).lang, Lang::of_session());
        fs::remove_file(&file)
    }

    #[test]
    fn a_language_is_found_by_its_own_name_and_by_its_english_one() {
        let ukrainian = Lang::Uk.name().to_lowercase();
        assert!(ukrainian.contains("українська") && ukrainian.contains("ukrainian"));
        assert_eq!(Lang::En.name(), "English");
    }
}
