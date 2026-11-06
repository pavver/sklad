//! Checks against the live system. Everything here only reads; run with `--ignored`.

use std::process::Command;

use sklad::i18n::Lang;
use sklad::model::{Reason, Removal, SourceKind, Usage};
use sklad::plan::Mode;
use sklad::scan::{self, Places};

fn pacman_list(args: &[&str], target: &str) -> Result<Vec<String>, String> {
    let output = Command::new("pacman")
        .args(args)
        .args(["--print-format", "%n", target])
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut list: Vec<String> = text.lines().map(str::to_owned).collect();
    list.sort();
    Ok(list)
}

/// The planner must agree with pacman on every package of the machine.
#[test]
#[ignore = "reads the live system"]
fn the_plan_agrees_with_pacman_on_every_package() {
    let catalog = scan::run(&Places::system(), Lang::Uk);
    let mut same = 0;
    let mut metas = 0;
    let mut differences = Vec::new();
    for item in catalog
        .items
        .iter()
        .filter(|item| item.removal == Removal::Pacman)
    {
        let Some(plan) = catalog.db.plan(&item.package, Mode::Keep) else {
            differences.push(format!("{}: no plan", item.package));
            continue;
        };
        if !plan.metas.is_empty() {
            metas += 1;
            continue;
        }
        let verdict = pacman_list(&["-Rsp"], &item.package);
        match (plan.is_blocked(), verdict) {
            (true, Err(_)) => same += 1,
            (false, Ok(list)) if list == plan.remove => same += 1,
            (false, Ok(list)) => {
                let extra: Vec<_> = plan.remove.iter().filter(|n| !list.contains(n)).collect();
                let missing: Vec<_> = list.iter().filter(|n| !plan.remove.contains(n)).collect();
                differences.push(format!("{}: ours+ {extra:?} pacman+ {missing:?}", item.package));
            }
            (true, Ok(_)) => differences.push(format!(
                "{}: we block {:?}, pacman does not",
                item.package, plan.blockers
            )),
            (false, Err(error)) => {
                differences.push(format!("{}: pacman refuses: {}", item.package, error.trim()))
            }
        }
    }
    println!(
        "agree {same}, through a metapackage {metas}, differ {}",
        differences.len()
    );
    for line in differences.iter().take(40) {
        println!("  {line}");
    }
    assert!(differences.is_empty());
}

/// The list passed by name, as the real removal does, is accepted by pacman as it stands.
#[test]
#[ignore = "reads the live system"]
fn plans_through_a_metapackage_are_accepted_by_pacman() {
    let catalog = scan::run(&Places::system(), Lang::Uk);
    let mut checked = 0;
    let mut refused = Vec::new();
    for item in catalog
        .items
        .iter()
        .filter(|item| item.removal == Removal::Pacman)
    {
        let Some(plan) = catalog.db.plan(&item.package, Mode::Keep) else {
            continue;
        };
        if plan.metas.is_empty() || plan.is_blocked() {
            continue;
        }
        checked += 1;
        let output = Command::new("pacman")
            .args(["-Rp", "--print-format", "%n"])
            .args(&plan.remove)
            .env("LC_ALL", "C")
            .output();
        if !output.is_ok_and(|output| output.status.success()) {
            refused.push(item.package.clone());
        }
    }
    println!("checked {checked}, refused {refused:?}");
    assert!(refused.is_empty());
}

#[test]
#[ignore = "reads the live system"]
fn languages_are_in_the_catalogue_and_their_programs_are_not_listed_twice() {
    let started = std::time::Instant::now();
    let _ = scan::langs::run(&Places::system(), &scan::langs::Owned::default(), Lang::Uk);
    println!("languages took {:?}", started.elapsed());
    let catalog = scan::run(&Places::system(), Lang::Uk);
    for item in &catalog.items {
        let kind = item.source.kind();
        if matches!(kind, SourceKind::Node | SourceKind::Python | SourceKind::Rustup) {
            let spec = sklad::remove::other_spec(&item.removal).map(|spec| spec.shown);
            println!(
                "{:?} | {} | {} | {:?} | {:?}",
                kind, item.title, item.version, item.size, spec
            );
            println!("    {} | {:?} | {:?}", item.summary, item.licenses, item.warning);
        }
        if kind == SourceKind::Manual {
            println!("Manual | {}", item.title);
        }
    }
    let of_kind = |kind: SourceKind, title: &str| {
        catalog
            .items
            .iter()
            .any(|item| item.source.kind() == kind && item.title == title)
    };
    assert!(!(of_kind(SourceKind::Node, "pnpm") && of_kind(SourceKind::Manual, "pnpm")));
}

#[test]
#[ignore = "reads the live system"]
fn the_catalogue_describes_the_machine() {
    let started = std::time::Instant::now();
    let catalog = scan::run(&Places::system(), Lang::Uk);
    println!("scan took {:?}, {} items", started.elapsed(), catalog.items.len());
    for kind in SourceKind::ALL {
        let count = catalog.items.iter().filter(|i| i.source.kind() == kind).count();
        println!("{:>12}: {count}", (Lang::Uk.text().source)(kind));
    }
    let apps = catalog.items.iter().filter(|i| i.section.is_app()).count();
    let with_icon = catalog
        .items
        .iter()
        .filter(|i| i.section.is_app() && i.icon.is_some())
        .count();
    let launched = catalog
        .items
        .iter()
        .filter(|i| matches!(i.usage, Usage::Last { .. }))
        .count();
    let running = catalog.items.iter().filter(|i| i.usage == Usage::Running).count();
    let explicit = catalog
        .items
        .iter()
        .filter(|i| i.reason == Some(Reason::Explicit))
        .count();
    let orphans = catalog.items.iter().filter(|i| i.is_orphan()).count();
    println!("apps {apps}, with icon {with_icon}, launch record {launched}, running {running}");
    println!("explicit {explicit}, orphans {orphans}");
    let long = catalog
        .items
        .iter()
        .filter(|i| i.description != i.summary && !i.description.is_empty())
        .count();
    println!("long descriptions {long}");
    for item in catalog
        .items
        .iter()
        .filter(|i| i.section.is_app() && i.icon.is_none())
        .take(25)
    {
        println!(
            "  no icon: {} [{}] {}",
            item.title,
            item.package,
            Lang::Uk.text().source_label(&item.source)
        );
    }
    for item in catalog
        .items
        .iter()
        .filter(|i| i.source.kind() == SourceKind::Manual)
    {
        println!(
            "  manual: {} | {} | {:?} | {:?}",
            item.title,
            (Lang::Uk.text().section)(item.section),
            item.usage,
            item.removal
        );
    }
    assert!(!catalog.items.is_empty());
}
