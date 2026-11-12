//! Interface words live only in the catalogue: a string left in the code stays Ukrainian in the
//! English window, and no window test sees it when it is painted without an accessibility label.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(folder: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, found)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(())
}

fn is_ukrainian(symbol: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&symbol)
}

#[test]
fn no_interface_word_lies_outside_the_catalogue() -> std::io::Result<()> {
    let mut files = Vec::new();
    rust_files(Path::new("src"), &mut files)?;
    let mut strays = Vec::new();
    for file in files.iter().filter(|file| !file.starts_with("src/i18n")) {
        let text = fs::read_to_string(file)?;
        let code = text.split("#[cfg(test)]").next().unwrap_or_default();
        for (number, line) in code.lines().enumerate() {
            if line.chars().any(is_ukrainian) {
                strays.push(format!("{}:{}: {}", file.display(), number + 1, line.trim()));
            }
        }
    }
    assert!(
        strays.is_empty(),
        "{} strays:\n{}",
        strays.len(),
        strays.join("\n")
    );
    Ok(())
}

#[test]
fn the_english_catalogue_is_english() -> std::io::Result<()> {
    let text = fs::read_to_string("src/i18n/en.rs")?;
    let strays: Vec<&str> = text
        .lines()
        .filter(|line| line.chars().any(is_ukrainian))
        .collect();
    assert!(strays.is_empty(), "{strays:?}");
    Ok(())
}
