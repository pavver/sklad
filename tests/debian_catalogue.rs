//! On Debian and Ubuntu the catalogue comes from the dpkg database, and its packages go through apt.

use std::fs;

use sklad::i18n::Lang;
use sklad::model::{PackageSystem, Reason, Removal, Source};
use sklad::plan::Mode;
use sklad::scan::{self, Places};

const STATUS: &str = "\
Package: hello
Status: install ok installed
Priority: optional
Section: utils
Installed-Size: 100
Architecture: amd64
Version: 2.10-3
Depends: libhello (>= 1.0)
Description: The classic greeting
 It prints a friendly greeting.

Package: libhello
Status: install ok installed
Priority: optional
Section: libs
Installed-Size: 50
Architecture: amd64
Version: 1.0-1
Description: The greeting library

Package: dpkg
Status: install ok installed
Essential: yes
Priority: required
Architecture: amd64
Version: 1.22
";

#[test]
fn a_dpkg_system_lists_its_packages_for_apt() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::temp_dir().join(format!("sklad-debian-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(base.join("dpkg/info"))?;
    fs::create_dir_all(base.join("apt"))?;
    fs::create_dir_all(base.join("home"))?;
    fs::create_dir_all(base.join("usr/share/applications"))?;
    fs::write(base.join("os-release"), "ID=debian\n")?;
    fs::write(base.join("dpkg/status"), STATUS)?;
    fs::write(
        base.join("apt/extended_states"),
        "Package: libhello\nArchitecture: amd64\nAuto-Installed: 1\n",
    )?;
    fs::write(
        base.join("dpkg/info/hello.list"),
        "/.\n/usr\n/usr/share\n/usr/share/applications\n/usr/share/applications/hello.desktop\n",
    )?;
    fs::write(
        base.join("usr/share/applications/hello.desktop"),
        "[Desktop Entry]\nType=Application\nName=Hello\nExec=hello\nCategories=Utility;\n",
    )?;
    let places = Places {
        home: base.join("home"),
        pacman_local: base.join("pacman"),
        dpkg_status: base.join("dpkg/status"),
        apt_extended_states: base.join("apt/extended_states"),
        os_release: base.join("os-release"),
        root: base.clone(),
    };
    let catalog = scan::run(&places, Lang::En);
    let _ = fs::remove_dir_all(&base);

    assert_eq!(catalog.system, Some(PackageSystem::Dpkg));
    let hello = catalog
        .items
        .iter()
        .find(|item| item.package == "hello")
        .ok_or("hello is listed")?;
    assert_eq!(hello.id, "dpkg:hello");
    assert_eq!(hello.title, "Hello");
    assert_eq!(hello.source, Source::LocalDeb, "no repository has it");
    assert_eq!(hello.removal, Removal::Package(PackageSystem::Dpkg));
    assert_eq!(hello.reason, Some(Reason::Explicit));
    assert_eq!(hello.description, "It prints a friendly greeting.");
    assert!(hello.installed.is_some(), "the time of its file list");
    let library = catalog
        .items
        .iter()
        .find(|item| item.package == "libhello")
        .ok_or("libhello is listed")?;
    assert_eq!(library.reason, Some(Reason::Dependency));
    let plan = catalog.db.plan("hello", Mode::Keep).ok_or("hello has a plan")?;
    assert_eq!(plan.remove, ["hello", "libhello"]);
    assert!(plan.critical.is_empty());
    Ok(())
}
