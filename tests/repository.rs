//! The repository carries nothing that only works on one machine and nothing that must stay private:
//! no paths of a real home, no dependencies from a local folder, no keys, tokens or stray files.

use std::fs;
use std::path::{Path, PathBuf};

/// Folders that are built or kept by tools rather than written.
const SKIPPED: [&str; 3] = [".git", "target", "dist"];

/// Text files that the checks read; binary assets have nothing to find.
const TEXT: [&str; 15] = [
    "rs", "toml", "md", "py", "svg", "desktop", "xml", "yml", "yaml", "lock", "sh", "html", "css", "txt",
    "asc",
];

fn files(folder: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let path = entry?.path();
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned());
        if path.is_dir() {
            if !name.is_some_and(|name| SKIPPED.contains(&name.as_str())) {
                files(&path, found)?;
            }
        } else {
            found.push(path);
        }
    }
    Ok(())
}

fn all_files() -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    files(Path::new("."), &mut found)?;
    found.sort();
    Ok(found)
}

fn is_text(path: &Path) -> bool {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned());
    path.extension()
        .is_some_and(|extension| TEXT.contains(&extension.to_string_lossy().as_ref()))
        || name.is_some_and(|name| ["PKGBUILD", ".SRCINFO", ".gitignore", "LICENSE"].contains(&name.as_str()))
}

/// Every text file with its lines, for the checks that look inside.
fn text_files() -> std::io::Result<Vec<(PathBuf, String)>> {
    let mut texts = Vec::new();
    for path in all_files()?.into_iter().filter(|path| is_text(path)) {
        // This file names the patterns it looks for.
        if path.ends_with("tests/repository.rs") {
            continue;
        }
        texts.push((path.clone(), fs::read_to_string(&path)?));
    }
    Ok(texts)
}

/// `name:line: text` for every line where `found` sees something.
fn lines_where(texts: &[(PathBuf, String)], found: impl Fn(&str) -> bool) -> Vec<String> {
    texts
        .iter()
        .flat_map(|(path, text)| {
            text.lines()
                .enumerate()
                .filter(|(_, line)| found(line))
                .map(move |(number, line)| format!("{}:{}: {}", path.display(), number + 1, line.trim()))
        })
        .collect()
}

/// Whether `line` holds `prefix` followed by at least `length` characters a token is made of.
fn has_token(line: &str, prefix: &str, length: usize) -> bool {
    line.match_indices(prefix).any(|(at, _)| {
        line[at + prefix.len()..]
            .chars()
            .take_while(|symbol| symbol.is_ascii_alphanumeric() || *symbol == '_' || *symbol == '-')
            .count()
            >= length
    })
}

/// A home folder of a real person. Tests use the invented `/home/me`.
fn has_real_home(line: &str) -> bool {
    let unix = line.match_indices("/home/").any(|(at, _)| {
        let user: String = line[at + "/home/".len()..]
            .chars()
            .take_while(|symbol| symbol.is_ascii_alphanumeric() || *symbol == '_' || *symbol == '-')
            .collect();
        !user.is_empty() && user != "me"
    });
    unix || line.contains("/Users/") || line.contains("C:\\Users\\")
}

fn has_secret(line: &str) -> bool {
    line.contains("PRIVATE KEY-----")
        || has_token(line, "ghp_", 30)
        || has_token(line, "gho_", 30)
        || has_token(line, "ghs_", 30)
        || has_token(line, "github_pat_", 30)
        || has_token(line, "glpat-", 20)
        || has_token(line, "sk-ant-", 20)
        || has_token(line, "sk-proj-", 20)
        || has_token(line, "xoxb-", 20)
        || has_token(line, "xoxp-", 20)
        || has_token(line, "AKIA", 16)
        || has_token(line, "AIza", 35)
        || has_token(line, "npm_", 30)
}

fn report(what: &str, found: &[String]) {
    assert!(found.is_empty(), "{what}:\n{}", found.join("\n"));
}

#[test]
fn no_path_points_into_a_real_home() -> std::io::Result<()> {
    let texts = text_files()?;
    report(
        "paths of a real machine",
        &lines_where(&texts, |line| {
            has_real_home(line) || line.contains("file:///home")
        }),
    );
    Ok(())
}

#[test]
fn no_key_or_token_is_committed() -> std::io::Result<()> {
    let texts = text_files()?;
    report("possible secrets", &lines_where(&texts, has_secret));
    Ok(())
}

#[test]
fn no_file_holds_private_settings_or_build_output() -> std::io::Result<()> {
    let forbidden = |path: &Path| {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned())
            .unwrap_or_default();
        name == ".env"
            || name.starts_with(".env.")
            || name == "credentials.json"
            || name.starts_with("id_rsa")
            || name.starts_with("id_ed25519")
            || ["pem", "key", "p12", "pfx", "kdbx", "orig", "rej", "swp"].contains(&extension.as_str())
            || name.ends_with(".pkg.tar.zst")
    };
    let found: Vec<String> = all_files()?
        .iter()
        .filter(|path| forbidden(path))
        .map(|path| path.display().to_string())
        .collect();
    report("private or generated files", &found);
    Ok(())
}

#[test]
fn no_file_is_large_enough_to_be_a_mistake() -> std::io::Result<()> {
    const LIMIT: u64 = 1024 * 1024;
    let mut found = Vec::new();
    for path in all_files()? {
        let size = fs::metadata(&path)?.len();
        if size > LIMIT {
            found.push(format!("{}: {size} bytes", path.display()));
        }
    }
    report("files over 1 MB", &found);
    Ok(())
}

/// A dependency from a local folder or an unpinned branch builds only on the machine that has it.
#[test]
fn every_dependency_comes_from_a_registry_or_a_pinned_public_commit() -> std::io::Result<()> {
    let mut found = Vec::new();
    for manifest in MANIFESTS {
        let manifest = fs::read_to_string(manifest)?;
        found.extend(local_dependencies(&manifest));
    }
    report("dependencies that only build here", &found);
    Ok(())
}

/// The manifests of the app and of the website.
const MANIFESTS: [&str; 2] = ["Cargo.toml", "site/Cargo.toml"];

/// Lines of a manifest that take a crate from a local folder, a patch, or an unpinned repository.
fn local_dependencies(manifest: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with("[patch") || line.contains("path =") || line.contains("path=") {
            found.push(line.to_owned());
        }
        if line.contains("git =")
            && (!line.contains("git = \"https://github.com/") || !line.contains("rev = \""))
        {
            found.push(line.to_owned());
        }
    }
    found
}

/// Markdown links to files of the repository stay relative, so they work in forks and on any branch.
#[test]
fn markdown_links_to_the_repository_are_relative() -> std::io::Result<()> {
    let markdown: Vec<(PathBuf, String)> = text_files()?
        .into_iter()
        .filter(|(path, _)| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    let found = lines_where(&markdown, |line| {
        line.contains("](/")
            || line.contains("github.com/pavver/sklad/blob/")
            || line.contains("github.com/pavver/sklad/tree/")
    });
    report("absolute links to files of the repository", &found);
    Ok(())
}

#[test]
fn the_checks_see_what_they_look_for() {
    assert!(has_real_home("let path = \"/home/alice/projects\";"));
    assert!(!has_real_home("PathBuf::from(\"/home/me/.local/bin/tool\")"));
    assert!(!has_real_home("home.join(\".local/share\")"));
    assert!(has_secret(&format!("token = \"ghp_{}\"", "a".repeat(36))));
    assert!(has_secret(&format!("-----BEGIN OPENSSH {}", "PRIVATE KEY-----")));
    assert!(!has_secret("the tokens crate, sk-ant- alone"));
    assert_eq!(local_dependencies("ui = { path = \"../ui\" }").len(), 1);
    assert_eq!(
        local_dependencies("ui = { git = \"https://github.com/a/b\" }").len(),
        1
    );
    assert!(local_dependencies("ui = { git = \"https://github.com/a/b\", rev = \"1\" }").is_empty());
}

/// The value after `key` up to the closing quote, on the first line that has it.
fn quoted_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let start = text.find(key)? + key.len();
    let rest = &text[start..];
    Some(&rest[..rest.find('"')?])
}

/// A release is tagged once: the crate, the pacman package and the software centres name one version.
#[test]
fn every_place_names_the_same_version() -> std::io::Result<()> {
    let manifest = fs::read_to_string("Cargo.toml")?;
    let pkgbuild = fs::read_to_string("packaging/arch/PKGBUILD")?;
    let metainfo = fs::read_to_string("packaging/io.github.pavver.sklad.metainfo.xml")?;
    let crate_version = quoted_after(&manifest, "\nversion = \"");
    let package_version = pkgbuild
        .lines()
        .find_map(|line| line.strip_prefix("pkgver="))
        .map(str::trim);
    let latest_release = quoted_after(&metainfo, "<release version=\"");
    assert!(crate_version.is_some());
    assert_eq!(crate_version, package_version, "PKGBUILD");
    assert_eq!(
        crate_version, latest_release,
        "the newest release of the metainfo"
    );
    let site = fs::read_to_string("site/Cargo.toml")?;
    assert_eq!(
        crate_version,
        quoted_after(&site, "\nversion = \""),
        "the website crate"
    );
    for page in ["site/index.html", "site/uk.html"] {
        let page_text = fs::read_to_string(page)?;
        let page_version = quoted_after(&page_text, "\"softwareVersion\": \"");
        assert_eq!(crate_version, page_version, "{page}");
    }
    Ok(())
}
