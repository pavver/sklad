//! Steam games: the manifests hold the name, the size and the time of the last launch.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Game {
    pub app_id: String,
    pub name: String,
    pub size: Option<u64>,
    pub last_played: Option<i64>,
}

/// A `"key" "value"` line of Valve's key-value format.
fn pair(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.split('"');
    parts.next()?;
    let key = parts.next()?;
    parts.next()?;
    Some((key, parts.next()?))
}

pub fn parse_manifest(text: &str) -> Option<Game> {
    let mut game = Game::default();
    for (key, value) in text.lines().filter_map(pair) {
        match key {
            "appid" if game.app_id.is_empty() => game.app_id = value.to_owned(),
            "name" if game.name.is_empty() => game.name = value.to_owned(),
            "SizeOnDisk" => game.size = value.parse().ok(),
            "LastPlayed" => game.last_played = value.parse().ok().filter(|time| *time > 0),
            _ => {}
        }
    }
    (!game.app_id.is_empty() && !game.name.is_empty()).then_some(game)
}

/// The library folders Steam knows, with the default one first.
pub fn libraries(steam_root: &Path) -> Vec<PathBuf> {
    let mut folders = vec![steam_root.to_path_buf()];
    let file = steam_root.join("steamapps/libraryfolders.vdf");
    let text = fs::read_to_string(file).unwrap_or_default();
    for (key, value) in text.lines().filter_map(pair) {
        let folder = PathBuf::from(value);
        if key == "path" && !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    folders
}

pub fn read_all(steam_root: &Path) -> Vec<Game> {
    let mut games: Vec<Game> = Vec::new();
    for folder in libraries(steam_root) {
        let Ok(entries) = fs::read_dir(folder.join("steamapps")) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "acf") {
                continue;
            }
            let game = fs::read_to_string(&path).ok().and_then(|t| parse_manifest(&t));
            if let Some(game) = game.filter(|g| !games.iter().any(|known| known.app_id == g.app_id)) {
                games.push(game);
            }
        }
    }
    games
}

/// Compatibility layers and runtimes are installed by Steam, not by the person.
pub fn is_tool(name: &str) -> bool {
    ["Proton", "Steam Linux Runtime", "Steamworks Common"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_is_read() {
        let text = "\"AppState\"\n{\n\t\"appid\"\t\t\"105600\"\n\t\"name\"\t\t\"Terraria\"\n\
            \t\"LastPlayed\"\t\t\"1785460625\"\n\t\"SizeOnDisk\"\t\t\"831365954\"\n}\n";
        let game = parse_manifest(text).unwrap_or_default();
        assert_eq!(game.app_id, "105600");
        assert_eq!(game.name, "Terraria");
        assert_eq!(game.size, Some(831365954));
        assert_eq!(game.last_played, Some(1785460625));
    }

    #[test]
    fn a_game_never_started_has_no_last_launch() {
        let text = "\"appid\" \"1\"\n\"name\" \"x\"\n\"LastPlayed\" \"0\"\n";
        assert_eq!(parse_manifest(text).and_then(|g| g.last_played), None);
    }

    #[test]
    fn the_tools_of_steam_are_told_from_games() {
        assert!(is_tool("Proton 9.0") && !is_tool("Terraria"));
    }
}
