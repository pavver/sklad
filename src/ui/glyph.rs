//! Glyphs of the icon font.

use crate::model::{Section, SourceKind};

pub const ARROW_CLOCKWISE: &str = "\u{E036}";
pub const ARROW_DOWN: &str = "\u{E03E}";
pub const ARROW_UP: &str = "\u{E08E}";
pub const CHECK: &str = "\u{E182}";
pub const COPY: &str = "\u{E1CA}";
pub const GLOBE: &str = "\u{E288}";
pub const MAGNIFYING_GLASS: &str = "\u{E30C}";
pub const MOON: &str = "\u{E330}";
pub const PACKAGE: &str = "\u{E390}";
pub const SQUARES_FOUR: &str = "\u{E464}";
pub const STACK: &str = "\u{E466}";
pub const SUN: &str = "\u{E472}";
pub const TRASH: &str = "\u{E4A6}";
pub const WARNING: &str = "\u{E4E0}";
pub const X: &str = "\u{E4F6}";

pub fn of_section(section: Section) -> &'static str {
    match section {
        Section::Internet => GLOBE,
        Section::Multimedia => "\u{E792}",
        Section::Graphics => "\u{E6F0}",
        Section::Office => "\u{E0EE}",
        Section::Development => "\u{E1BC}",
        Section::Games => "\u{E26E}",
        Section::Education => "\u{E62C}",
        Section::Science => "\u{E5E4}",
        Section::System => "\u{E270}",
        Section::Utilities => "\u{E5D4}",
        Section::Console => "\u{EAE8}",
        Section::Libraries => "\u{E758}",
        Section::Fonts => "\u{E6EE}",
        Section::Drivers => "\u{E610}",
        Section::Meta => STACK,
    }
}

pub fn of_source(source: SourceKind) -> &'static str {
    match source {
        SourceKind::Repo => PACKAGE,
        SourceKind::Aur => "\u{E5D4}",
        SourceKind::Flatpak => STACK,
        SourceKind::Steam => "\u{E26E}",
        SourceKind::Cargo | SourceKind::Node | SourceKind::Python => "\u{E1BC}",
        SourceKind::Rustup => "\u{E610}",
        SourceKind::Manual => "\u{E1C8}",
    }
}
