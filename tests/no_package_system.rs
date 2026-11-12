//! A distribution whose packages Sklad cannot read yet still gets the rest of the catalogue.

use std::fs;
use std::path::PathBuf;

use sklad::i18n::Lang;
use sklad::model::SourceKind;
use sklad::scan::{self, Places};

#[test]
fn without_pacman_or_dpkg_the_other_sources_are_still_listed() -> std::io::Result<()> {
    let base = std::env::temp_dir().join(format!("sklad-no-system-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let tool = base.join("home/.local/share/pipx/venvs/black");
    fs::create_dir_all(&tool)?;
    fs::write(tool.join("pyvenv.cfg"), "")?;
    fs::write(base.join("os-release"), "ID=fedora\n")?;
    let places = Places {
        home: base.join("home"),
        pacman_local: base.join("pacman"),
        dpkg_status: base.join("dpkg/status"),
        os_release: base.join("os-release"),
        root: PathBuf::from(&base),
    };
    let catalog = scan::run(&places, Lang::En);
    let _ = fs::remove_dir_all(&base);

    assert_eq!(catalog.system, None);
    assert!(
        catalog
            .items
            .iter()
            .all(|item| !matches!(item.source.kind(), SourceKind::Repo | SourceKind::Aur)),
        "no packages of a system that is not there"
    );
    assert!(
        catalog.items.iter().any(|item| item.package == "black"),
        "the pipx tool is listed"
    );
    Ok(())
}
