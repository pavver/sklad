//! What the package managers of programming languages installed: npm, pnpm, pip, pipx, uv, rustup.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Places, blank, loose};
use crate::i18n::{Lang, Text};
use crate::model::{Access, Item, Removal, Section, Source};

/// What `package.json` says about a package.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodePackage {
    pub name: String,
    pub version: String,
    pub summary: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub author: Option<String>,
    pub has_programs: bool,
}

/// A field that is either a plain string or an object holding it under a key.
fn text_or_field(value: Option<&serde_json::Value>, key: &str) -> Option<String> {
    let value = value?;
    value
        .as_str()
        .or_else(|| value.get(key).and_then(|inner| inner.as_str()))
        .map(str::to_owned)
}

pub fn parse_package_json(text: &str) -> Option<NodePackage> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let field = |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    Some(NodePackage {
        name: field("name")?,
        version: field("version").unwrap_or_default(),
        summary: field("description").unwrap_or_default(),
        license: text_or_field(value.get("license"), "type"),
        homepage: field("homepage"),
        author: text_or_field(value.get("author"), "name"),
        has_programs: value.get("bin").is_some(),
    })
}

/// Package folders of a `node_modules`, with scoped ones one level deeper.
pub fn node_packages_in(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for folder in loose::directories_in(root) {
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.starts_with('@') {
            found.extend(loose::directories_in(&folder));
        } else if !name.starts_with('.') {
            found.push(folder);
        }
    }
    found
}

/// What the core metadata of a Python package says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PythonPackage {
    pub name: String,
    pub version: String,
    pub summary: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub author: Option<String>,
}

/// The headers of `METADATA` or `PKG-INFO`. A blank line starts the long text, which is not read.
pub fn parse_metadata(text: &str) -> Option<PythonPackage> {
    let mut package = PythonPackage::default();
    let mut expression = None;
    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        let Some((key, value)) = line.split_once(": ") else {
            continue;
        };
        let value = value.trim().to_owned();
        let known = value != "UNKNOWN";
        match key {
            "Name" => package.name = value,
            "Version" => package.version = value,
            "Summary" if known => package.summary = value,
            "License-Expression" => expression = Some(value),
            "License" if known => package.license = Some(value),
            "Home-page" if known => package.homepage = Some(value),
            "Project-URL" => {
                let url = value
                    .split_once(", ")
                    .filter(|(label, _)| label.eq_ignore_ascii_case("homepage"));
                if let Some((_, url)) = url {
                    package.homepage = Some(url.to_owned());
                }
            }
            "Author" if known => package.author = Some(value),
            "Author-email" if package.author.is_none() => {
                let name = value
                    .split(['<', ','])
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                package.author = Some(name).filter(|name| !name.is_empty());
            }
            _ => {}
        }
    }
    package.license = expression.or(package.license);
    (!package.name.is_empty()).then_some(package)
}

fn is_python_record(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "dist-info" || extension == "egg-info")
}

fn read_python_record(path: &Path) -> Option<PythonPackage> {
    let file = if path.is_file() {
        path.to_path_buf()
    } else if path.extension().is_some_and(|extension| extension == "dist-info") {
        path.join("METADATA")
    } else {
        path.join("PKG-INFO")
    };
    parse_metadata(&fs::read_to_string(file).ok()?)
}

/// The `site-packages` folders under a `lib` folder, with the interpreter each belongs to.
fn site_packages_in(lib: &Path) -> Vec<SitePackages> {
    loose::directories_in(lib)
        .into_iter()
        .filter_map(|folder| {
            let interpreter = folder.file_name()?.to_str()?.to_owned();
            let path = folder.join("site-packages");
            (interpreter.starts_with("python") && path.is_dir()).then_some(SitePackages { path, interpreter })
        })
        .collect()
}

struct SitePackages {
    path: PathBuf,
    interpreter: String,
}

/// The default toolchain from `settings.toml`.
pub fn default_toolchain(settings: &str) -> Option<String> {
    settings.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "default_toolchain").then(|| value.trim().trim_matches('"').to_owned())
    })
}

/// The version of a toolchain from the channel manifest it was installed with. A custom toolchain,
/// linked in by hand, has no such file.
pub fn toolchain_version(manifest: &str) -> Option<String> {
    let mut lines = manifest.lines().skip_while(|line| line.trim() != "[pkg.rust]");
    lines.find_map(|line| {
        let (key, value) = line.split_once('=')?;
        let version = value.trim().trim_matches('"').split(' ').next()?;
        (key.trim() == "version").then(|| version.to_owned())
    })
}

/// `stable-x86_64-unknown-linux-gnu` -> `stable`, `1.88.0-x86_64-unknown-linux-gnu` -> `1.88.0`.
pub fn toolchain_title(name: &str) -> &str {
    name.split_once("-x86_64")
        .or_else(|| name.split_once("-aarch64"))
        .map_or(name, |(channel, _)| channel)
}

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

fn node_item(folder: &Path, package: NodePackage, source_name: &str, removal: Removal) -> Item {
    let id = format!("{source_name}:{}", folder.display());
    let mut item = blank(id, package.name.clone(), Source::Node, removal);
    item.version = package.version;
    item.summary = package.summary.clone();
    item.description = package.summary;
    item.licenses = package.license.into_iter().collect();
    item.homepage = package
        .homepage
        .or(Some(format!("https://www.npmjs.com/package/{}", package.name)));
    item.developer = package.author;
    item.size = Some(loose::size_of(folder));
    item.section = if package.has_programs {
        Section::Console
    } else {
        Section::Libraries
    };
    item
}

fn read_node(folder: &Path) -> Option<NodePackage> {
    parse_package_json(&fs::read_to_string(folder.join("package.json")).ok()?)
}

fn npm_items(places: &Places, owned: &Owned, out: &mut Scan) {
    let user_prefix = places.home.join(".local");
    let user_root = user_prefix.join("lib/node_modules");
    for folder in node_packages_in(&user_root) {
        let Some(package) = read_node(&folder) else {
            continue;
        };
        let prefix = user_prefix.display().to_string();
        let name = package.name.clone();
        let argv = words(&["npm", "uninstall", "-g", "--prefix", &prefix, &name]);
        out.items.push(node_item(
            &folder,
            package,
            "npm",
            manager("npm", argv, Access::User),
        ));
    }
    out.claimed.push(user_root);

    let system_root = places.root.join("usr/lib/node_modules");
    for folder in node_packages_in(&system_root) {
        let relative = folder
            .strip_prefix(&system_root)
            .map(Path::to_path_buf)
            .unwrap_or_default();
        if owned.node.contains(&relative.display().to_string()) {
            continue;
        }
        let Some(package) = read_node(&folder) else {
            continue;
        };
        let name = package.name.clone();
        let argv = words(&["npm", "uninstall", "-g", &name]);
        out.items.push(node_item(
            &folder,
            package,
            "npm",
            manager("npm", argv, Access::Root),
        ));
    }
}

fn pnpm_items(places: &Places, out: &mut Scan) {
    let global = places.home.join(".local/share/pnpm/global");
    for version in loose::directories_in(&global) {
        for folder in node_packages_in(&version.join("node_modules")) {
            let Some(package) = read_node(&folder) else {
                continue;
            };
            let argv = vec![
                tool(places, "pnpm"),
                "remove".to_owned(),
                "-g".to_owned(),
                package.name.clone(),
            ];
            out.items.push(node_item(
                &folder,
                package,
                "pnpm",
                manager("pnpm", argv, Access::User),
            ));
        }
    }
    out.claimed.push(global);
}

fn python_item(record: &Path, package: PythonPackage, removal: Removal) -> Item {
    let id = format!("python:{}", record.display());
    let mut item = blank(id, package.name.clone(), Source::Python, removal);
    item.version = package.version;
    item.summary = package.summary.clone();
    item.description = package.summary;
    item.licenses = package.license.into_iter().collect();
    item.homepage = package
        .homepage
        .or(Some(format!("https://pypi.org/project/{}", package.name)));
    item.developer = package.author;
    item.section = Section::Libraries;
    item
}

fn pip_items(places: &Places, owned: &Owned, text: &Text, out: &mut Scan) {
    for site in site_packages_in(&places.home.join(".local/lib")) {
        for record in loose::entries_in(&site.path)
            .iter()
            .filter(|path| is_python_record(path))
        {
            let Some(package) = read_python_record(record) else {
                continue;
            };
            let argv = words(&[
                &site.interpreter,
                "-m",
                "pip",
                "uninstall",
                "-y",
                "--break-system-packages",
                &package.name,
            ]);
            out.items
                .push(python_item(record, package, manager("pip", argv, Access::User)));
        }
    }
    for lib in ["usr/lib", "usr/local/lib"] {
        for site in site_packages_in(&places.root.join(lib)) {
            for record in loose::entries_in(&site.path)
                .iter()
                .filter(|path| is_python_record(path))
            {
                let name = record
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if owned.python.contains(&name) {
                    continue;
                }
                let Some(package) = read_python_record(record) else {
                    continue;
                };
                let reason = (text.stray_python)(&record.display().to_string());
                out.items
                    .push(python_item(record, package, Removal::Unavailable { reason }));
            }
        }
    }
}

/// Isolated environments, one folder per tool, as `pipx` and `uv tool` keep them.
fn environment_items(places: &Places, text: &Text, out: &mut Scan) {
    let kinds = [
        (".local/share/pipx/venvs", "pipx", ["uninstall"].as_slice()),
        (".local/share/uv/tools", "uv", ["tool", "uninstall"].as_slice()),
    ];
    for (folder, program, verb) in kinds {
        let root = places.home.join(folder);
        for environment in loose::directories_in(&root) {
            let name = environment
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut argv = vec![tool(places, program)];
            argv.extend(verb.iter().map(|word| (*word).to_owned()));
            argv.push(name.clone());
            let id = format!("{program}:{name}");
            let mut item = blank(
                id,
                name.clone(),
                Source::Python,
                manager(program, argv, Access::User),
            );
            item.summary = (text.python_tool)(program);
            item.description = item.summary.clone();
            item.homepage = Some(format!("https://pypi.org/project/{name}"));
            item.size = Some(loose::size_of(&environment));
            out.items.push(item);
        }
        out.claimed.push(root);
    }
}

fn uv_pythons(places: &Places, text: &Text, out: &mut Scan) {
    let root = places.home.join(".local/share/uv/python");
    for folder in loose::directories_in(&root) {
        let is_link = fs::symlink_metadata(&folder).is_ok_and(|meta| meta.file_type().is_symlink());
        let key = folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if is_link || !key.starts_with("cpython-") && !key.starts_with("pypy-") {
            continue;
        }
        let mut parts = key.split('-');
        let implementation = parts.next().unwrap_or_default();
        let version = parts.next().unwrap_or_default().to_owned();
        let argv = vec![
            tool(places, "uv"),
            "python".to_owned(),
            "uninstall".to_owned(),
            key.clone(),
        ];
        let id = format!("uv-python:{key}");
        let mut item = blank(id, key.clone(), Source::Python, manager("uv", argv, Access::User));
        item.title = format!("Python {version} ({implementation})");
        item.version = version;
        item.summary = text.uv_python.to_owned();
        item.description = item.summary.clone();
        item.homepage = Some("https://www.python.org".to_owned());
        item.size = Some(loose::size_of(&folder));
        out.items.push(item);
    }
    out.claimed.push(root);
}

fn rustup_items(places: &Places, text: &Text, out: &mut Scan) {
    let root = places.home.join(".rustup");
    let settings = fs::read_to_string(root.join("settings.toml")).unwrap_or_default();
    let default = default_toolchain(&settings);
    for folder in loose::directories_in(&root.join("toolchains")) {
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let argv = words(&["rustup", "toolchain", "uninstall", &name]);
        let id = format!("rustup:{name}");
        let mut item = blank(
            id,
            name.clone(),
            Source::Rustup,
            manager("rustup", argv, Access::User),
        );
        item.title = format!("Rust {}", toolchain_title(&name));
        let manifest = fs::read_to_string(folder.join("lib/rustlib/multirust-channel-manifest.toml"));
        item.version = manifest
            .ok()
            .as_deref()
            .and_then(toolchain_version)
            .unwrap_or_default();
        item.summary = text.rust_toolchain.to_owned();
        item.description = item.summary.clone();
        item.homepage = Some("https://www.rust-lang.org".to_owned());
        item.size = Some(loose::size_of(&folder));
        if default
            .as_deref()
            .is_some_and(|chosen| chosen == name || chosen == toolchain_title(&name))
        {
            item.warning = Some(text.default_toolchain.to_owned());
        }
        out.items.push(item);
    }
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
