//! Launcher files (`.desktop`): the name, the icon and the section of a program with a window.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::i18n::Lang;
use crate::model::Section;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Launcher {
    /// The file name without `.desktop`: the key Plasma records launches under.
    pub id: String,
    /// Where the file lies; empty for a launcher parsed from text alone.
    pub path: PathBuf,
    pub name: String,
    pub comment: Option<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub exec: Option<String>,
    pub shown: bool,
}

pub fn read(path: &Path, lang: Lang) -> Option<Launcher> {
    let bytes = fs::read(path).ok()?;
    let id = path.file_stem()?.to_string_lossy().into_owned();
    let mut launcher = parse(&id, &String::from_utf8_lossy(&bytes), lang)?;
    launcher.path = path.to_path_buf();
    Some(launcher)
}

pub fn parse(id: &str, text: &str, lang: Lang) -> Option<Launcher> {
    let fields = entry_fields(text);
    if fields.get("Type").copied() != Some("Application") {
        return None;
    }
    let field = |key: &str| fields.get(key).map(|value| (*value).to_owned());
    let local = |key: &str| field(&format!("{key}[{}]", lang.code())).or_else(|| field(key));
    let hidden = ["NoDisplay", "Hidden"]
        .iter()
        .any(|key| fields.get(key) == Some(&"true"));
    Some(Launcher {
        id: id.to_owned(),
        name: local("Name")?,
        comment: local("Comment").filter(|comment| !comment.is_empty()),
        icon: field("Icon").filter(|icon| !icon.is_empty()),
        exec: field("Exec"),
        categories: fields
            .get("Categories")
            .map(|list| {
                list.split(';')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        shown: !hidden,
        path: PathBuf::new(),
    })
}

/// The keys and values of the `[Desktop Entry]` group, trimmed; a later key wins.
fn entry_fields(text: &str) -> HashMap<&str, &str> {
    let mut fields = HashMap::new();
    let mut in_entry = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
        } else if let Some((key, value)) = line.split_once('=').filter(|_| in_entry) {
            fields.insert(key.trim(), value.trim());
        }
    }
    fields
}

/// The section a launcher belongs to. The order settles programs that list several categories.
pub fn section_of(categories: &[String]) -> Section {
    const RULES: [(&str, Section); 14] = [
        ("Game", Section::Games),
        ("WebBrowser", Section::Internet),
        ("Network", Section::Internet),
        ("Development", Section::Development),
        ("Graphics", Section::Graphics),
        ("AudioVideo", Section::Multimedia),
        ("Audio", Section::Multimedia),
        ("Video", Section::Multimedia),
        ("Office", Section::Office),
        ("Education", Section::Education),
        ("Science", Section::Science),
        ("System", Section::System),
        ("Settings", Section::System),
        ("Utility", Section::Utilities),
    ];
    RULES
        .iter()
        .find(|(name, _)| categories.iter().any(|category| category == name))
        .map_or(Section::Utilities, |(_, section)| *section)
}

/// The program an `Exec=` line starts, without `env`, its variables and the field codes.
pub fn exec_program(exec: &str) -> Option<String> {
    exec.split_whitespace()
        .map(|token| token.trim_matches('"'))
        .find(|token| *token != "env" && !token.contains('=') && !token.starts_with('%'))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOLPHIN: &str = "[Desktop Entry]\nType=Application\nName=Dolphin\nName[uk]=Дельфін\n\
        Comment=Files\nComment[uk]=Файли\nIcon=system-file-manager\nExec=dolphin %u\n\
        Categories=Qt;KDE;System;FileManager;\n[Desktop Action New]\nName=Other\nIcon=other\n";

    #[test]
    fn an_english_reader_gets_the_plain_name() {
        let launcher = parse("org.kde.dolphin", DOLPHIN, Lang::En).unwrap_or_default();
        assert_eq!(launcher.name, "Dolphin");
        assert_eq!(launcher.comment.as_deref(), Some("Files"));
    }

    #[test]
    fn the_translated_name_wins_and_actions_are_ignored() {
        let launcher = parse("org.kde.dolphin", DOLPHIN, Lang::Uk).unwrap_or_default();
        assert_eq!(launcher.name, "Дельфін");
        assert_eq!(launcher.comment.as_deref(), Some("Файли"));
        assert_eq!(launcher.icon.as_deref(), Some("system-file-manager"));
        assert_eq!(section_of(&launcher.categories), Section::System);
    }

    #[test]
    fn a_link_is_not_a_program() {
        assert!(parse("x", "[Desktop Entry]\nType=Link\nName=x\n", Lang::Uk).is_none());
    }

    #[test]
    fn a_hidden_launcher_is_marked() {
        let text = "[Desktop Entry]\nType=Application\nName=x\nNoDisplay=true\n";
        assert_eq!(parse("x", text, Lang::Uk).map(|l| l.shown), Some(false));
    }

    #[test]
    fn the_program_is_found_behind_env() {
        let exec = "env FOO=1 /opt/app/run --flag %U";
        assert_eq!(exec_program(exec).as_deref(), Some("/opt/app/run"));
    }

    #[test]
    fn a_game_stays_a_game_when_it_also_says_utility() {
        let categories = vec!["Utility".to_owned(), "Game".to_owned()];
        assert_eq!(section_of(&categories), Section::Games);
    }
}
