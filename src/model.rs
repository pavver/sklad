//! What the catalogue holds: one `Item` per thing installed, whatever put it there.

use std::path::PathBuf;

/// Where an item came from. The repository name is kept because it is what the person recognises.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Repo(String),
    Aur,
    Flatpak,
    Steam,
    Cargo,
    Node,
    Python,
    Rustup,
    /// A Debian package installed from a file, which no repository has.
    LocalDeb,
    Manual,
}

/// The package manager a distribution is built on. Sources that belong to one appear only there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PackageSystem {
    /// Arch Linux and the distributions based on it.
    Pacman,
    /// Debian, Ubuntu and the distributions based on them.
    Dpkg,
}

impl PackageSystem {
    /// The tool that installs and removes packages, as the person knows it.
    pub fn manager(self) -> &'static str {
        match self {
            PackageSystem::Pacman => "pacman",
            PackageSystem::Dpkg => "apt",
        }
    }
}

/// `Source` without its payload, for the filter in the side bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Repo,
    Aur,
    Flatpak,
    Steam,
    Cargo,
    Node,
    Python,
    Rustup,
    LocalDeb,
    Manual,
}

impl Source {
    pub fn kind(&self) -> SourceKind {
        match self {
            Source::Repo(_) => SourceKind::Repo,
            Source::Aur => SourceKind::Aur,
            Source::Flatpak => SourceKind::Flatpak,
            Source::Steam => SourceKind::Steam,
            Source::Cargo => SourceKind::Cargo,
            Source::Node => SourceKind::Node,
            Source::Python => SourceKind::Python,
            Source::Rustup => SourceKind::Rustup,
            Source::LocalDeb => SourceKind::LocalDeb,
            Source::Manual => SourceKind::Manual,
        }
    }
}

impl SourceKind {
    pub const ALL: [SourceKind; 10] = [
        SourceKind::Repo,
        SourceKind::Aur,
        SourceKind::Flatpak,
        SourceKind::Steam,
        SourceKind::Cargo,
        SourceKind::Node,
        SourceKind::Python,
        SourceKind::Rustup,
        SourceKind::LocalDeb,
        SourceKind::Manual,
    ];

    /// Whether the source can exist on a system built on `system`: the repositories need a package
    /// system of the distribution, the AUR needs pacman, local Debian packages need dpkg, and the
    /// rest work anywhere.
    pub fn fits(self, system: Option<PackageSystem>) -> bool {
        match self {
            SourceKind::Repo => system.is_some(),
            SourceKind::Aur => system == Some(PackageSystem::Pacman),
            SourceKind::LocalDeb => system == Some(PackageSystem::Dpkg),
            _ => true,
        }
    }

    /// Language managers install command-line tools and libraries, never windowed programs.
    pub fn holds_only_tools(self) -> bool {
        matches!(
            self,
            SourceKind::Cargo | SourceKind::Node | SourceKind::Python | SourceKind::Rustup
        )
    }
}

/// The sections of the side bar. The first group holds programs with a window, the second the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Internet,
    Multimedia,
    Graphics,
    Office,
    Development,
    Games,
    Education,
    Science,
    System,
    Utilities,
    Console,
    Libraries,
    Fonts,
    Drivers,
    Meta,
}

impl Section {
    pub const APPS: [Section; 10] = [
        Section::Internet,
        Section::Multimedia,
        Section::Graphics,
        Section::Office,
        Section::Development,
        Section::Games,
        Section::Education,
        Section::Science,
        Section::System,
        Section::Utilities,
    ];

    pub const OTHER: [Section; 5] = [
        Section::Console,
        Section::Libraries,
        Section::Fonts,
        Section::Drivers,
        Section::Meta,
    ];

    pub fn is_app(self) -> bool {
        Self::APPS.contains(&self)
    }
}

/// Why a package is on the system, in pacman's terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Explicit,
    Dependency,
}

/// What is known about the last launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Usage {
    /// A process of this program is alive right now.
    Running,
    /// A launch was recorded at this Unix time.
    Last { at: i64, by: Evidence },
    /// A program that would be recorded, with no record of a launch.
    NoRecord,
    /// Launches of this kind of item are not recorded anywhere, so nothing can be said.
    NotTracked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    Plasma,
    Steam,
}

/// Who may touch a path that is removed as plain files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    User,
    Root,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlatpakScope {
    System,
    User,
}

/// How a program with a window is started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Launch {
    /// Its launcher file, started the way the menu starts it.
    Desktop(PathBuf),
    /// A game, started by the Steam client.
    Steam { app_id: String },
}

/// How the item leaves the system. Packages of the distribution are planned against the package
/// graph and removed by its package manager.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Removal {
    Package(PackageSystem),
    Flatpak {
        app_id: String,
        scope: FlatpakScope,
    },
    Steam {
        app_id: String,
    },
    Cargo {
        name: String,
    },
    Files {
        paths: Vec<PathBuf>,
        access: Access,
    },
    /// The uninstall command of the manager that put the item there.
    Manager {
        name: String,
        argv: Vec<String>,
        access: Access,
    },
    /// Shown in the catalogue, with the reason it cannot be removed from here.
    Unavailable {
        reason: String,
    },
}

#[derive(Clone, Debug)]
pub struct Item {
    /// Stable across rescans: the key of the selection.
    pub id: String,
    pub title: String,
    pub package: String,
    pub version: String,
    pub summary: String,
    pub description: String,
    pub licenses: Vec<String>,
    pub homepage: Option<String>,
    pub developer: Option<String>,
    pub source: Source,
    pub section: Section,
    pub reason: Option<Reason>,
    pub size: Option<u64>,
    pub installed: Option<i64>,
    pub usage: Usage,
    pub icon: Option<PathBuf>,
    pub launchers: Vec<String>,
    /// `None` for anything without a window to open.
    pub launch: Option<Launch>,
    pub groups: Vec<String>,
    pub depends: Vec<String>,
    pub required_by: Vec<String>,
    pub removal: Removal,
    /// Said in the removal dialog before anything happens.
    pub warning: Option<String>,
    /// Lower-cased title, package and summary: what the search box looks through.
    pub haystack: String,
}

impl Item {
    /// A dependency nothing needs any more.
    pub fn is_orphan(&self) -> bool {
        self.reason == Some(Reason::Dependency) && self.required_by.is_empty()
    }

    pub fn seal(mut self) -> Self {
        self.haystack = format!("{} {} {}", self.title, self.package, self.summary).to_lowercase();
        self
    }
}
