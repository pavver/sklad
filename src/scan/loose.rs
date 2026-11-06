//! What no package manager knows about: programs copied by hand, launchers, `cargo install`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A crate installed by `cargo install`, from `~/.cargo/.crates2.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoCrate {
    pub name: String,
    pub version: String,
    pub binaries: Vec<String>,
}

pub fn parse_crates(json: &str) -> Vec<CargoCrate> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(installs) = value.get("installs").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    installs
        .iter()
        .filter_map(|(key, entry)| {
            let mut words = key.split_whitespace();
            let name = words.next()?.to_owned();
            let version = words.next()?.to_owned();
            let binaries = entry
                .get("bins")?
                .as_array()?
                .iter()
                .filter_map(|bin| bin.as_str().map(str::to_owned))
                .collect();
            Some(CargoCrate {
                name,
                version,
                binaries,
            })
        })
        .collect()
}

/// Regular files of a directory: what can be started from it.
pub fn files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
}

/// Every entry of a directory, sorted.
pub fn entries_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    paths
}

pub fn directories_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    folders
}

/// Every `.desktop` file below a directory: Wine keeps its launchers in nested folders.
pub fn launchers_below(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(folder) = pending.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "desktop") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Bytes under a path, without following links.
pub fn size_of(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    let mut total = 0;
    let mut pending = vec![path.to_path_buf()];
    while let Some(folder) = pending.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                pending.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }
    total
}

/// Which file a name in a launcher points at, given the files already found.
pub fn find_program<'a>(program: &str, known: &'a HashMap<String, PathBuf>) -> Option<&'a PathBuf> {
    let name = Path::new(program).file_name()?.to_str()?;
    known.get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crates_are_read() {
        let json =
            r#"{"installs":{"espflash 3.1.0 (registry+https://x)":{"bins":["espflash","cargo-espflash"]}}}"#;
        let crates = parse_crates(json);
        assert_eq!(crates.len(), 1);
        assert_eq!(crates[0].name, "espflash");
        assert_eq!(crates[0].version, "3.1.0");
        assert_eq!(crates[0].binaries, ["espflash", "cargo-espflash"]);
    }

    #[test]
    fn a_broken_file_gives_nothing_and_does_not_stop_the_scan() {
        assert!(parse_crates("not json").is_empty());
    }
}
