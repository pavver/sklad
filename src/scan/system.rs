//! Which package system the distribution is built on, read from the system itself.
//!
//! The rules, in order:
//! 1. `/etc/os-release` names the distribution in `ID` and the ones it is based on in `ID_LIKE`.
//!    `arch` means pacman; `debian` or `ubuntu` means dpkg.
//! 2. The named system counts only when its database is there.
//! 3. Without a usable name, the one database present decides.
//! 4. Otherwise there is no package system: the catalogue holds the sources found anywhere,
//!    such as Flatpak, Steam and the language managers.

use crate::model::PackageSystem;

use super::Places;

/// The package system `os-release` names, through `ID` or `ID_LIKE`.
fn named(os_release: &str) -> Option<PackageSystem> {
    let mut words = Vec::new();
    for line in os_release.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "ID" || key.trim() == "ID_LIKE" {
            let value = value.trim().trim_matches(['"', '\'']);
            words.extend(value.split_whitespace().map(str::to_lowercase));
        }
    }
    if words.iter().any(|word| word == "arch") {
        Some(PackageSystem::Pacman)
    } else if words.iter().any(|word| word == "debian" || word == "ubuntu") {
        Some(PackageSystem::Dpkg)
    } else {
        None
    }
}

/// The rules of this module over what was read: the name and which databases exist.
pub fn choose(os_release: &str, has_pacman: bool, has_dpkg: bool) -> Option<PackageSystem> {
    let present = |system| match system {
        PackageSystem::Pacman => has_pacman,
        PackageSystem::Dpkg => has_dpkg,
    };
    match named(os_release) {
        Some(system) if present(system) => Some(system),
        _ => match (has_pacman, has_dpkg) {
            (true, false) => Some(PackageSystem::Pacman),
            (false, true) => Some(PackageSystem::Dpkg),
            _ => None,
        },
    }
}

pub fn detect(places: &Places) -> Option<PackageSystem> {
    let os_release = std::fs::read_to_string(&places.os_release).unwrap_or_default();
    choose(
        &os_release,
        places.pacman_local.is_dir(),
        places.dpkg_status.is_file(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCH: &str = "NAME=\"Arch Linux\"\nID=arch\n";
    const CACHY: &str = "NAME=\"CachyOS Linux\"\nID=cachyos\nID_LIKE=arch\n";
    const MINT: &str = "NAME=\"Linux Mint\"\nID=linuxmint\nID_LIKE=\"ubuntu debian\"\n";
    const DEBIAN: &str = "ID=debian\n";
    const FEDORA: &str = "ID=fedora\n";

    #[test]
    fn a_distribution_follows_the_one_it_is_based_on() {
        assert_eq!(choose(ARCH, true, false), Some(PackageSystem::Pacman));
        assert_eq!(choose(CACHY, true, false), Some(PackageSystem::Pacman));
        assert_eq!(choose(MINT, false, true), Some(PackageSystem::Dpkg));
        assert_eq!(choose(DEBIAN, false, true), Some(PackageSystem::Dpkg));
    }

    #[test]
    fn the_name_settles_a_system_with_both_databases() {
        // dpkg can be installed on Arch, for instance to unpack a .deb.
        assert_eq!(choose(ARCH, true, true), Some(PackageSystem::Pacman));
        assert_eq!(choose(DEBIAN, true, true), Some(PackageSystem::Dpkg));
        assert_eq!(choose(FEDORA, true, true), None);
    }

    #[test]
    fn a_name_without_its_database_does_not_count() {
        assert_eq!(choose(ARCH, false, false), None);
        assert_eq!(choose(DEBIAN, true, false), Some(PackageSystem::Pacman));
    }

    #[test]
    fn an_unknown_distribution_is_judged_by_its_database() {
        assert_eq!(choose(FEDORA, false, false), None);
        assert_eq!(choose("", true, false), Some(PackageSystem::Pacman));
        assert_eq!(choose(FEDORA, false, true), Some(PackageSystem::Dpkg));
    }
}
