//! Python packages installed by pip, pipx and uv.

use std::fs;
use std::path::{Path, PathBuf};

use super::{Owned, Scan, manager, words};
use crate::i18n::Text;
use crate::model::{Access, Item, Removal, Section, Source};
use crate::scan::{Places, blank, loose};

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
    let headers = text
        .lines()
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(": "));
    for (key, value) in headers {
        let value = value.trim().to_owned();
        if key == "License-Expression" {
            expression = Some(value);
        } else {
            read_header(&mut package, key, value);
        }
    }
    package.license = expression.or(package.license);
    (!package.name.is_empty()).then_some(package)
}

/// One header into the package. `UNKNOWN` means the field was left out.
pub(super) fn read_header(package: &mut PythonPackage, key: &str, value: String) {
    let known = value != "UNKNOWN";
    match key {
        "Name" => package.name = value,
        "Version" => package.version = value,
        "Summary" if known => package.summary = value,
        "License" if known => package.license = Some(value),
        "Home-page" if known => package.homepage = Some(value),
        "Project-URL" => package.homepage = project_homepage(&value).or(package.homepage.take()),
        "Author" if known => package.author = Some(value),
        "Author-email" if package.author.is_none() => package.author = author_name(&value),
        _ => {}
    }
}

/// The address of a `Project-URL` labelled as the homepage.
pub(super) fn project_homepage(value: &str) -> Option<String> {
    let (label, url) = value.split_once(", ")?;
    label.eq_ignore_ascii_case("homepage").then(|| url.to_owned())
}

/// The name before the address in `Author-email`, `Name <name@host>`.
pub(super) fn author_name(value: &str) -> Option<String> {
    let name = value.split(['<', ',']).next().unwrap_or_default().trim();
    (!name.is_empty()).then(|| name.to_owned())
}

pub(super) fn is_python_record(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "dist-info" || extension == "egg-info")
}

pub(super) fn read_python_record(path: &Path) -> Option<PythonPackage> {
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
pub(super) fn site_packages_in(lib: &Path) -> Vec<SitePackages> {
    loose::directories_in(lib)
        .into_iter()
        .filter_map(|folder| {
            let interpreter = folder.file_name()?.to_str()?.to_owned();
            let path = folder.join("site-packages");
            (interpreter.starts_with("python") && path.is_dir()).then_some(SitePackages { path, interpreter })
        })
        .collect()
}

pub(super) struct SitePackages {
    path: PathBuf,
    interpreter: String,
}

pub(super) fn python_item(record: &Path, package: PythonPackage, removal: Removal) -> Item {
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

pub(super) fn pip_items(places: &Places, owned: &Owned, text: &Text, out: &mut Scan) {
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
