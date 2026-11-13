//! Rust toolchains installed by rustup.

use std::fs;

use super::{Scan, manager, words};
use crate::i18n::Text;
use crate::model::{Access, Source};
use crate::scan::{Places, blank, loose};

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

pub(super) fn rustup_items(places: &Places, text: &Text, out: &mut Scan) {
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
