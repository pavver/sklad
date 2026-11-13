//! Files outside every package manager: loose programs, folders in `/opt`, and the launchers the
//! person made, which join the program they start.

use std::fs;
use std::path::{Path, PathBuf};

use sklad::i18n::Lang;
use sklad::model::{Access, Item, Removal, Section};
use sklad::scan::{self, Places};

fn write(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, content)
}

fn launcher(name: &str, exec: &str, extra: &str) -> String {
    format!("[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\n{extra}")
}

fn find<'a>(items: &'a [Item], title: &str) -> Option<&'a Item> {
    items.iter().find(|item| item.title == title)
}

fn removed_paths(item: &Item) -> Vec<PathBuf> {
    match &item.removal {
        Removal::Files { paths, .. } => paths.clone(),
        _ => Vec::new(),
    }
}

#[test]
fn loose_programs_and_launchers_become_entries() -> std::io::Result<()> {
    let base = std::env::temp_dir().join(format!("sklad-loose-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let home = base.join("home");
    let applications = home.join(".local/share/applications");
    write(&home.join(".local/bin/mytool"), "#!/bin/sh\n")?;
    write(
        &applications.join("mytool.desktop"),
        &launcher("My Tool", "mytool --gui", ""),
    )?;
    write(
        &applications.join("web/site.desktop"),
        &launcher("A Site", "/nowhere/app", ""),
    )?;
    write(
        &applications.join("hidden.desktop"),
        &launcher("Hidden", "/x", "NoDisplay=true\n"),
    )?;
    write(
        &applications.join("game.desktop"),
        &launcher("Game", "steam steam://rungameid/1", ""),
    )?;
    write(&base.join("usr/local/bin/sysbin"), "")?;
    fs::create_dir_all(base.join("opt/bigapp"))?;
    write(&base.join("os-release"), "ID=fedora\n")?;
    let places = Places {
        home: home.clone(),
        pacman_local: base.join("pacman"),
        dpkg_status: base.join("dpkg/status"),
        apt_extended_states: base.join("apt/extended_states"),
        os_release: base.join("os-release"),
        root: PathBuf::from(&base),
    };
    let catalog = scan::run(&places, Lang::En);
    let _ = fs::remove_dir_all(&base);
    let items = &catalog.items;

    let tool = find(items, "My Tool").ok_or_else(|| std::io::Error::other("no entry for mytool"))?;
    assert_eq!(
        removed_paths(tool),
        [
            home.join(".local/bin/mytool"),
            applications.join("mytool.desktop")
        ],
        "the launcher joins the program it starts, and goes with it"
    );
    assert!(find(items, "mytool").is_none(), "one entry, not two");

    let site = find(items, "A Site").ok_or_else(|| std::io::Error::other("no entry for the site"))?;
    assert_eq!(removed_paths(site), [applications.join("web/site.desktop")]);

    assert!(find(items, "Hidden").is_none(), "a hidden launcher is no program");
    assert!(find(items, "Game").is_none(), "Steam lists its games itself");

    let system = find(items, "sysbin").ok_or_else(|| std::io::Error::other("no /usr/local/bin entry"))?;
    assert!(matches!(
        system.removal,
        Removal::Files {
            access: Access::Root,
            ..
        }
    ));
    let opt = find(items, "bigapp").ok_or_else(|| std::io::Error::other("no /opt entry"))?;
    assert_eq!(opt.section, Section::Utilities);
    assert!(matches!(
        opt.removal,
        Removal::Files {
            access: Access::Root,
            ..
        }
    ));
    Ok(())
}
