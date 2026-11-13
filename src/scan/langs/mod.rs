//! What the package managers of programming languages installed: npm, pnpm, pip, pipx, uv, rustup,
//! and the Python environments of virtualenvwrapper, pyenv, conda and mamba.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{Places, blank, loose};
use crate::i18n::Lang;
use crate::model::{Access, Item, Removal, Source};

mod environments;
mod node;
mod python;
mod rust;

use environments::{environment_items, known_environments, uv_pythons};
use node::{npm_items, pnpm_items};
use python::pip_items;
use rust::rustup_items;

/// A tool by its full path when it lives in `~/.local/bin`: a program started from the menu may not
/// have that folder on its `PATH`.
fn tool(places: &Places, name: &str) -> String {
    let local = places.home.join(".local/bin").join(name);
    if local.is_file() {
        local.display().to_string()
    } else {
        name.to_owned()
    }
}

fn manager(name: &str, argv: Vec<String>, access: Access) -> Removal {
    Removal::Manager {
        name: name.to_owned(),
        argv,
        access,
    }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| (*word).to_owned()).collect()
}

/// An environment or an interpreter that goes to the bin as it lies.
fn trash_item(paths: Vec<PathBuf>, name: &str, version: Option<String>, summary: String) -> Item {
    let main = paths.last().cloned().unwrap_or_default();
    let removal = Removal::Files {
        paths,
        access: Access::User,
    };
    let mut item = blank(
        format!("venv:{}", main.display()),
        name.to_owned(),
        Source::Python,
        removal,
    );
    item.version = version.unwrap_or_default();
    item.summary = summary;
    item.description = item.summary.clone();
    item.size = Some(loose::size_of(&main));
    item
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// What pacman owns among the folders scanned here, so it is not listed twice.
#[derive(Clone, Debug, Default)]
pub struct Owned {
    /// Package folders relative to `/usr/lib/node_modules`.
    pub node: HashSet<String>,
    /// Names of `.dist-info` and `.egg-info` records.
    pub python: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Scan {
    pub items: Vec<Item>,
    /// Folders whose programs are already accounted for: a link into one is not a loose file.
    pub claimed: Vec<PathBuf>,
}

pub fn run(places: &Places, owned: &Owned, lang: Lang) -> Scan {
    let text = lang.text();
    let mut scan = Scan::default();
    npm_items(places, owned, &mut scan);
    pnpm_items(places, &mut scan);
    pip_items(places, owned, text, &mut scan);
    environment_items(places, text, &mut scan);
    known_environments(places, text, &mut scan);
    uv_pythons(places, text, &mut scan);
    rustup_items(places, text, &mut scan);
    scan.claimed = scan
        .claimed
        .iter()
        .filter_map(|path| path.canonicalize().ok())
        .collect();
    scan
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::model::Section;

    use super::node::parse_package_json;
    use super::python::parse_metadata;
    use super::rust::{default_toolchain, toolchain_title, toolchain_version};
    use super::*;
    use crate::model::Removal;
    use std::error::Error;

    /// A home and a root in a folder of their own, removed at the end of the test.
    struct Sandbox {
        base: PathBuf,
    }

    impl Sandbox {
        fn new(name: &str) -> Self {
            let base = std::env::temp_dir().join(format!("sklad-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&base);
            Self { base }
        }

        fn write(&self, path: &str, text: &str) -> std::io::Result<()> {
            let file = self.base.join(path);
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file, text)
        }

        fn places(&self) -> Places {
            Places {
                home: self.base.join("home"),
                pacman_local: self.base.join("pacman"),
                dpkg_status: self.base.join("dpkg/status"),
                apt_extended_states: self.base.join("apt/extended_states"),
                os_release: self.base.join("os-release"),
                root: self.base.join("root"),
            }
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn by_title<'a>(scan: &'a Scan, title: &str) -> Option<&'a Item> {
        scan.items.iter().find(|item| item.title == title)
    }

    fn argv_of(item: Option<&Item>) -> Vec<String> {
        match item.map(|item| &item.removal) {
            Some(Removal::Manager { argv, .. }) => argv.clone(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn a_scan_speaks_the_language_it_was_asked_for() -> Result<(), Box<dyn Error>> {
        let sandbox = Sandbox::new("language");
        sandbox.write(
            "home/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc",
            "",
        )?;
        sandbox.write("home/.rustup/settings.toml", "default_toolchain = \"stable\"\n")?;
        let scan = run(&sandbox.places(), &Owned::default(), Lang::En);
        let item = by_title(&scan, "Rust stable");
        assert_eq!(
            item.map(|item| item.summary.as_str()),
            Some(Lang::En.text().rust_toolchain)
        );
        let warning = item.and_then(|item| item.warning.as_deref());
        assert_eq!(warning, Some(Lang::En.text().default_toolchain));
        Ok(())
    }

    #[test]
    fn a_package_json_is_read_in_both_shapes_of_its_fields() {
        let text = r#"{"name":"@scope/tool","version":"1.2.0","description":"Does things",
            "license":{"type":"MIT"},"author":{"name":"Me"},"bin":{"tool":"cli.js"}}"#;
        let package = parse_package_json(text).unwrap_or_default();
        assert_eq!(package.name, "@scope/tool");
        assert_eq!(package.license.as_deref(), Some("MIT"));
        assert_eq!(package.author.as_deref(), Some("Me"));
        assert!(package.has_programs);
        let plain = r#"{"name":"x","license":"ISC","author":"You <you@x>"}"#;
        assert_eq!(
            parse_package_json(plain).and_then(|p| p.license).as_deref(),
            Some("ISC")
        );
    }

    #[test]
    fn python_metadata_prefers_the_expression_and_stops_at_the_text() {
        let text = "Metadata-Version: 2.4\nName: annotated-types\nVersion: 0.8.0\n\
            Summary: Constraint types\nProject-URL: Source, https://src\n\
            Project-URL: Homepage, https://home\nAuthor-email: Adrian <a@x>, Sam <s@x>\n\
            License: old text\nLicense-Expression: MIT\n\nName: not this\n";
        let package = parse_metadata(text).unwrap_or_default();
        assert_eq!(package.name, "annotated-types");
        assert_eq!(package.license.as_deref(), Some("MIT"));
        assert_eq!(package.homepage.as_deref(), Some("https://home"));
        assert_eq!(package.author.as_deref(), Some("Adrian"));
    }

    #[test]
    fn unknown_fields_of_old_metadata_stay_empty() {
        let package = parse_metadata("Name: x\nSummary: UNKNOWN\nLicense: UNKNOWN\n").unwrap_or_default();
        assert_eq!(package.summary, "");
        assert_eq!(package.license, None);
    }

    #[test]
    fn the_default_toolchain_and_short_titles_are_read() {
        let settings = "version = \"12\"\ndefault_toolchain = \"stable-x86_64-unknown-linux-gnu\"\n";
        assert_eq!(
            default_toolchain(settings).as_deref(),
            Some("stable-x86_64-unknown-linux-gnu")
        );
        assert_eq!(toolchain_title("1.88.0-x86_64-unknown-linux-gnu"), "1.88.0");
        assert_eq!(toolchain_title("esp"), "esp");
    }

    #[test]
    fn user_packages_are_found_with_the_command_that_removes_them() -> Result<(), Box<dyn Error>> {
        let sandbox = Sandbox::new("user");
        let package = r#"{"name":"tool","version":"1.0.0","bin":{"tool":"cli.js"}}"#;
        sandbox.write("home/.local/lib/node_modules/tool/package.json", package)?;
        let scoped = r#"{"name":"@me/lib","version":"2.0.0"}"#;
        sandbox.write("home/.local/lib/node_modules/@me/lib/package.json", scoped)?;
        let meta = "Name: requests\nVersion: 2.0\n";
        sandbox.write(
            "home/.local/lib/python3.12/site-packages/requests-2.0.dist-info/METADATA",
            meta,
        )?;
        sandbox.write("home/.local/share/pipx/venvs/black/pyvenv.cfg", "")?;
        sandbox.write("home/.local/share/uv/tools/ruff/pyvenv.cfg", "")?;
        sandbox.write(
            "home/.local/share/uv/python/cpython-3.12.16-linux-x86_64-gnu/bin/python",
            "",
        )?;
        sandbox.write(
            "home/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc",
            "",
        )?;
        sandbox.write(
            "home/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc",
            "",
        )?;
        sandbox.write(
            "home/.rustup/settings.toml",
            "default_toolchain = \"stable-x86_64-unknown-linux-gnu\"\n",
        )?;

        let places = sandbox.places();
        let scan = run(&places, &Owned::default(), Lang::Uk);
        let prefix = places.home.join(".local").display().to_string();

        let tool = by_title(&scan, "tool");
        assert_eq!(
            argv_of(tool),
            words(&["npm", "uninstall", "-g", "--prefix", &prefix, "tool"])
        );
        assert_eq!(tool.map(|item| item.section), Some(Section::Console));
        let lib = by_title(&scan, "@me/lib");
        assert_eq!(lib.map(|item| item.section), Some(Section::Libraries));
        assert_eq!(
            argv_of(by_title(&scan, "requests"))[..4],
            words(&["python3.12", "-m", "pip", "uninstall"])
        );
        assert_eq!(
            argv_of(by_title(&scan, "black")),
            words(&["pipx", "uninstall", "black"])
        );
        assert_eq!(
            argv_of(by_title(&scan, "ruff")),
            words(&["uv", "tool", "uninstall", "ruff"])
        );
        assert_eq!(
            argv_of(by_title(&scan, "Python 3.12.16 (cpython)")),
            words(&["uv", "python", "uninstall", "cpython-3.12.16-linux-x86_64-gnu"])
        );
        assert!(by_title(&scan, "Rust stable").is_some_and(|item| item.warning.is_some()));
        assert!(by_title(&scan, "Rust nightly").is_some_and(|item| item.warning.is_none()));
        Ok(())
    }

    fn trashed(item: Option<&Item>) -> Vec<PathBuf> {
        match item.map(|item| &item.removal) {
            Some(Removal::Files {
                paths,
                access: Access::User,
            }) => paths.clone(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn environments_in_known_places_go_to_the_bin_whole() -> Result<(), Box<dyn Error>> {
        let sandbox = Sandbox::new("environments");
        sandbox.write(
            "home/.virtualenvs/notes/pyvenv.cfg",
            "home = /usr/bin
version = 3.12.4
",
        )?;
        sandbox.write("home/.virtualenvs/not-an-env/readme", "")?;
        sandbox.write("home/.pyenv/versions/3.11.9/bin/python3", "")?;
        sandbox.write(
            "home/.pyenv/versions/3.11.9/envs/web/pyvenv.cfg",
            "version_info = 3.11.9.final.0
",
        )?;
        sandbox.write(
            "home/miniconda3/envs/science/conda-meta/python-3.10.14-h955ad1f_1.json",
            "{}",
        )?;
        sandbox.write("home/micromamba/envs/tiny/conda-meta/history", "")?;
        let places = sandbox.places();
        std::os::unix::fs::symlink(
            places.home.join(".pyenv/versions/3.11.9/envs/web"),
            places.home.join(".pyenv/versions/web"),
        )?;
        let scan = run(&places, &Owned::default(), Lang::En);
        let home = &places.home;

        let notes = by_title(&scan, "notes");
        assert_eq!(trashed(notes), [home.join(".virtualenvs/notes")]);
        assert_eq!(notes.map(|item| item.version.as_str()), Some("3.12.4"));
        assert_eq!(
            notes.map(|item| item.summary.as_str()),
            Some("A Python environment of virtualenvwrapper")
        );
        assert!(by_title(&scan, "not-an-env").is_none());

        let interpreter = by_title(&scan, "Python 3.11.9 (pyenv)");
        assert_eq!(trashed(interpreter), [home.join(".pyenv/versions/3.11.9")]);
        let web = by_title(&scan, "web");
        assert_eq!(
            trashed(web),
            [
                home.join(".pyenv/versions/web"),
                home.join(".pyenv/versions/3.11.9/envs/web")
            ],
            "the link pyenv keeps goes too"
        );
        assert_eq!(web.map(|item| item.version.as_str()), Some("3.11.9"));

        let science = by_title(&scan, "science");
        assert_eq!(trashed(science), [home.join("miniconda3/envs/science")]);
        assert_eq!(science.map(|item| item.version.as_str()), Some("3.10.14"));
        assert_eq!(
            by_title(&scan, "tiny").map(|item| item.summary.as_str()),
            Some("A Python environment of mamba")
        );
        Ok(())
    }

    #[test]
    fn the_version_of_a_toolchain_comes_from_the_rust_package_of_its_manifest() {
        let manifest =
            "[pkg.cargo]\nversion = \"0.90.0 (abc)\"\n[pkg.rust]\nversion = \"1.99.1 (58c 2026-10-13)\"\n";
        assert_eq!(toolchain_version(manifest).as_deref(), Some("1.99.1"));
        assert_eq!(toolchain_version("[pkg.cargo]\nversion = \"0.90.0\"\n"), None);
    }

    #[test]
    fn a_default_written_as_a_bare_channel_still_warns() -> Result<(), Box<dyn Error>> {
        let sandbox = Sandbox::new("channel");
        sandbox.write(
            "home/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc",
            "",
        )?;
        sandbox.write("home/.rustup/settings.toml", "default_toolchain = \"stable\"\n")?;
        let scan = run(&sandbox.places(), &Owned::default(), Lang::Uk);
        assert!(by_title(&scan, "Rust stable").is_some_and(|item| item.warning.is_some()));
        Ok(())
    }

    #[test]
    fn what_pacman_owns_is_not_listed_again() -> Result<(), Box<dyn Error>> {
        let sandbox = Sandbox::new("owned");
        sandbox.write("root/usr/lib/node_modules/npm/package.json", r#"{"name":"npm"}"#)?;
        sandbox.write(
            "root/usr/lib/node_modules/stray/package.json",
            r#"{"name":"stray"}"#,
        )?;
        let site = "root/usr/lib/python3.14/site-packages";
        sandbox.write(&format!("{site}/pip-26.dist-info/METADATA"), "Name: pip\n")?;
        sandbox.write(&format!("{site}/stale-1.egg-info/PKG-INFO"), "Name: stale\n")?;
        let owned = Owned {
            node: HashSet::from(["npm".to_owned()]),
            python: HashSet::from(["pip-26.dist-info".to_owned()]),
        };
        let scan = run(&sandbox.places(), &owned, Lang::Uk);
        assert!(by_title(&scan, "npm").is_none());
        assert!(by_title(&scan, "pip").is_none());
        let stray = by_title(&scan, "stray").map(|item| &item.removal);
        assert!(matches!(
            stray,
            Some(Removal::Manager {
                access: Access::Root,
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn a_stray_record_of_the_system_python_is_shown_but_never_offered_for_removal()
    -> Result<(), Box<dyn Error>> {
        let sandbox = Sandbox::new("stray");
        let record = "root/usr/lib/python3.14/site-packages/pyzfs-2.4.2-py3.14.egg-info/PKG-INFO";
        sandbox.write(record, "Name: pyzfs\nVersion: 2.4.2\n")?;
        let scan = run(&sandbox.places(), &Owned::default(), Lang::Uk);
        let removal = by_title(&scan, "pyzfs").map(|item| &item.removal);
        assert!(matches!(removal, Some(Removal::Unavailable { .. })));
        Ok(())
    }

    #[test]
    fn missing_folders_give_an_empty_result() {
        assert!(
            run(&Sandbox::new("empty").places(), &Owned::default(), Lang::Uk)
                .items
                .is_empty()
        );
    }
}
