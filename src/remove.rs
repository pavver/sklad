//! Turns an item into the command that removes it, and runs that command.

use std::io::{BufRead, BufReader, Read};
use std::path::{Component, Path};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use crate::model::{Access, DockerObject, FlatpakScope, Item, PackageSystem, Removal};
use crate::plan::{Mode, PackageDb, Plan};

/// A command in two forms: what is executed and what is shown to the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    pub argv: Vec<String>,
    pub shown: String,
}

fn quote(word: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "-_+.:/@=".contains(c);
    if !word.is_empty() && word.chars().all(plain) {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

fn quoted(words: &[String]) -> String {
    words.iter().map(|word| quote(word)).collect::<Vec<_>>().join(" ")
}

fn direct(argv: &[&str]) -> Spec {
    let argv: Vec<String> = argv.iter().map(|word| (*word).to_owned()).collect();
    let shown = quoted(&argv);
    Spec { argv, shown }
}

/// The removal of a plan by the package manager of the system.
pub fn package_spec(plan: &Plan, system: PackageSystem) -> Spec {
    match system {
        PackageSystem::Pacman => pacman_spec(plan),
        PackageSystem::Dpkg => apt_spec(plan),
    }
}

/// One removal run as root in a shell, the kept packages marked as installed by hand first.
/// `shown` is the command a person would type; `run` is the same with the flags that keep the
/// manager from asking.
fn root_spec(plan: &Plan, mark: &str, shown: &str, run: &str) -> Spec {
    let remove = quoted(&plan.remove);
    let steps = |sudo: &str, command: &str| {
        let removal = format!("{sudo}{command} {remove}");
        if plan.keep.is_empty() {
            removal
        } else {
            format!("{sudo}{mark} {} && {removal}", quoted(&plan.keep))
        }
    };
    let script = steps("", run);
    Spec {
        argv: ["pkexec", "/usr/bin/sh", "-c", script.as_str()]
            .map(str::to_owned)
            .to_vec(),
        shown: steps("sudo ", shown),
    }
}

/// `--noconfirm` is safe here only because the list passed by name is the list the person saw:
/// pacman refuses it if anything else would have to change.
fn pacman_spec(plan: &Plan) -> Spec {
    root_spec(
        plan,
        "pacman -D --asexplicit",
        "pacman -Rn",
        "pacman -Rn --noconfirm",
    )
}

/// apt removes exactly the packages named, since the plan already holds everything that needs
/// them; `-y` stays safe because the same list was simulated first, and apt still refuses an
/// essential package without a flag nobody passes. Configuration files stay, as with
/// `apt-get remove` by hand.
fn apt_spec(plan: &Plan) -> Spec {
    root_spec(
        plan,
        "apt-mark manual",
        "apt-get remove",
        "DEBIAN_FRONTEND=noninteractive apt-get remove -y",
    )
}

/// The packages `apt-get -s remove` would remove, from its `Remv` lines. A package it would
/// install instead comes back marked with `+`, so the list can never match a plan.
pub fn parse_apt_simulation(output: &str) -> Vec<String> {
    let mut list: Vec<String> = output
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            match (words.next(), words.next()) {
                (Some("Remv"), Some(name)) => Some(name.to_owned()),
                (Some("Inst"), Some(name)) => Some(format!("+{name}")),
                _ => None,
            }
        })
        .collect();
    list.sort();
    list
}

/// Refuses a path too close to the root to be a thing somebody installed.
pub fn path_is_removable(path: &Path) -> bool {
    let depth = path
        .components()
        .filter(|part| matches!(part, Component::Normal(_)))
        .count();
    let clean = !path.components().any(|part| matches!(part, Component::ParentDir));
    path.is_absolute() && clean && depth >= 2
}

fn manager_spec(argv: &[String], access: Access) -> Spec {
    match access {
        Access::User => Spec {
            argv: argv.to_vec(),
            shown: quoted(argv),
        },
        Access::Root => {
            let mut full = vec!["pkexec".to_owned()];
            full.extend_from_slice(argv);
            Spec {
                argv: full,
                shown: format!("sudo {}", quoted(argv)),
            }
        }
    }
}

/// The command for anything but pacman. `None` for a pacman item or a path refused above.
pub fn other_spec(removal: &Removal) -> Option<Spec> {
    match removal {
        Removal::Package(_) | Removal::Unavailable { .. } => None,
        Removal::Manager { argv, access, .. } => Some(manager_spec(argv, *access)),
        // `--noninteractive` also forbids the polkit dialog flatpak would raise for a system
        // installation, so that one runs through pkexec like any other command for root.
        Removal::Flatpak { app_id, scope } => {
            let (flag, access) = match scope {
                FlatpakScope::System => ("--system", Access::Root),
                FlatpakScope::User => ("--user", Access::User),
            };
            let argv = ["flatpak", "uninstall", "--noninteractive", flag, app_id].map(str::to_owned);
            Some(manager_spec(&argv, access))
        }
        Removal::Steam { app_id } => Some(direct(&["xdg-open", &format!("steam://uninstall/{app_id}")])),
        Removal::Cargo { name } => Some(direct(&["cargo", "uninstall", name])),
        Removal::Docker { object, id } => {
            let kind = match object {
                DockerObject::Container => "container",
                DockerObject::Image => "image",
                DockerObject::Volume => "volume",
            };
            Some(direct(&["docker", kind, "rm", id]))
        }
        Removal::Files { paths, access } => {
            if !paths.iter().all(|path| path_is_removable(path)) {
                return None;
            }
            let names: Vec<String> = paths.iter().map(|path| path.display().to_string()).collect();
            let head: &[&str] = match access {
                Access::User => &["gio", "trash", "--"],
                Access::Root => &["pkexec", "rm", "-rf", "--"],
            };
            let mut argv: Vec<String> = head.iter().map(|word| (*word).to_owned()).collect();
            argv.extend(names.iter().cloned());
            let shown = match access {
                Access::User => format!("gio trash -- {}", quoted(&names)),
                Access::Root => format!("sudo rm -rf -- {}", quoted(&names)),
            };
            Some(Spec { argv, shown })
        }
    }
}

/// The command a person could run by hand. `None` when other programs hold the package.
pub fn command_line(item: &Item, db: &PackageDb) -> Option<String> {
    match &item.removal {
        Removal::Package(system) => {
            let plan = db
                .plan(&item.package, Mode::Keep)
                .filter(|plan| !plan.is_blocked())?;
            Some(package_spec(&plan, *system).shown)
        }
        other => other_spec(other).map(|spec| spec.shown),
    }
}

/// True when the removal cannot be undone from the recycle bin or a reinstall. An image can be
/// pulled again; what a container or a volume holds cannot.
pub fn is_irreversible(item: &Item) -> bool {
    matches!(
        item.removal,
        Removal::Files {
            access: Access::Root,
            ..
        } | Removal::Docker {
            object: DockerObject::Container | DockerObject::Volume,
            ..
        }
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Line(String),
    Finished(Outcome),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Done,
    /// The password window was closed.
    Dismissed,
    Failed(Failure),
}

/// Why a command did not finish. The interface words it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Code(i32),
    Interrupted,
    EmptyCommand,
    NotStarted(String),
}

fn outcome_of(code: Option<i32>, program: &str) -> Outcome {
    match code {
        Some(0) => Outcome::Done,
        // pkexec exits with 126 when the dialog is dismissed and 127 when access is refused.
        Some(126 | 127) if program == "pkexec" => Outcome::Dismissed,
        Some(code) => Outcome::Failed(Failure::Code(code)),
        None => Outcome::Failed(Failure::Interrupted),
    }
}

/// What the interface needs from the system. Tests replace it, so no test touches a real package.
pub trait Backend: Send + Sync {
    /// The sorted list the package manager itself would remove for these names, or its refusal.
    fn preview(&self, system: PackageSystem, names: &[String]) -> Result<Vec<String>, String>;
    fn run(&self, spec: &Spec) -> Receiver<Event>;
}

pub struct SystemBackend;

fn forward(stream: impl Read + Send + 'static, sender: Sender<Event>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            if sender.send(Event::Line(line)).is_err() {
                return;
            }
        }
    })
}

pub fn run_streaming(spec: &Spec) -> Receiver<Event> {
    let (sender, receiver) = channel();
    let spec = spec.clone();
    thread::spawn(move || {
        let Some((program, args)) = spec.argv.split_first() else {
            let _ = sender.send(Event::Finished(Outcome::Failed(Failure::EmptyCommand)));
            return;
        };
        let child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(error) => {
                let _ = sender.send(Event::Finished(Outcome::Failed(Failure::NotStarted(
                    error.to_string(),
                ))));
                return;
            }
        };
        let readers = [
            child.stdout.take().map(|out| forward(out, sender.clone())),
            child.stderr.take().map(|err| forward(err, sender.clone())),
        ];
        let status = child.wait();
        for reader in readers.into_iter().flatten() {
            let _ = reader.join();
        }
        let outcome = match status {
            Ok(status) => outcome_of(status.code(), program),
            Err(error) => Outcome::Failed(Failure::NotStarted(error.to_string())),
        };
        let _ = sender.send(Event::Finished(outcome));
    });
    receiver
}

impl Backend for SystemBackend {
    /// `pacman --print` and `apt-get -s` change nothing and need no privileges.
    fn preview(&self, system: PackageSystem, names: &[String]) -> Result<Vec<String>, String> {
        let mut command = match system {
            PackageSystem::Pacman => {
                let mut command = Command::new("pacman");
                command.args(["-Rp", "--print-format", "%n"]);
                command
            }
            PackageSystem::Dpkg => {
                let mut command = Command::new("apt-get");
                command.args(["-s", "remove"]);
                command
            }
        };
        let output = command
            .args(names)
            .env("LC_ALL", "C")
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            Ok(match system {
                PackageSystem::Pacman => {
                    let mut list: Vec<String> = text.lines().map(str::to_owned).collect();
                    list.sort();
                    list
                }
                PackageSystem::Dpkg => parse_apt_simulation(&text),
            })
        } else {
            let text = String::from_utf8_lossy(&output.stderr);
            Err(text.trim().to_owned())
        }
    }

    fn run(&self, spec: &Spec) -> Receiver<Event> {
        run_streaming(spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn the_plain_removal_is_one_privileged_command() {
        let plan = Plan {
            remove: words(&["app", "lib"]),
            ..Plan::default()
        };
        let spec = pacman_spec(&plan);
        assert_eq!(spec.argv[0], "pkexec");
        assert_eq!(spec.argv[3], "pacman -Rn --noconfirm app lib");
        assert_eq!(spec.shown, "sudo pacman -Rn app lib");
    }

    #[test]
    fn the_siblings_are_marked_before_anything_is_removed() {
        let plan = Plan {
            remove: words(&["app", "bundle-meta"]),
            keep: words(&["other"]),
            ..Plan::default()
        };
        assert_eq!(
            pacman_spec(&plan).argv[3],
            "pacman -D --asexplicit other && pacman -Rn --noconfirm app bundle-meta"
        );
    }

    #[test]
    fn a_name_cannot_smuggle_a_command_into_the_script() {
        let plan = Plan {
            remove: words(&["a; rm -rf /"]),
            ..Plan::default()
        };
        assert!(pacman_spec(&plan).argv[3].ends_with("'a; rm -rf /'"));
    }

    #[test]
    fn apt_marks_the_siblings_and_removes_exactly_the_plan() {
        let plan = Plan {
            remove: words(&["gimp", "gimp-meta"]),
            keep: words(&["inkscape"]),
            ..Plan::default()
        };
        let spec = package_spec(&plan, PackageSystem::Dpkg);
        assert_eq!(spec.argv[0], "pkexec");
        assert_eq!(
            spec.argv[3],
            "apt-mark manual inkscape && DEBIAN_FRONTEND=noninteractive apt-get remove -y gimp gimp-meta"
        );
        assert_eq!(
            spec.shown,
            "sudo apt-mark manual inkscape && sudo apt-get remove gimp gimp-meta"
        );
    }

    #[test]
    fn the_apt_simulation_names_what_goes_and_marks_what_would_come() {
        let output = "NOTE: This is only a simulation!\nReading package lists...\n\
            Remv gimp [2.10.34-1]\nRemv libgimp2.0:i386 [2.10.34-1]\nInst gawk (1:5.2.1-2 Debian:12/stable)\n\
            Conf gawk (1:5.2.1-2 Debian:12/stable)\n";
        assert_eq!(
            parse_apt_simulation(output),
            words(&["+gawk", "gimp", "libgimp2.0:i386"])
        );
    }

    #[test]
    fn a_path_at_the_root_is_refused() {
        for path in ["/", "/opt", "/home", "relative/path", "/opt/../etc/x"] {
            assert!(!path_is_removable(Path::new(path)), "{path}");
        }
        assert!(path_is_removable(Path::new("/opt/app")));
        assert!(path_is_removable(Path::new("/home/me/.local/bin/x")));
    }

    #[test]
    fn a_refused_path_gets_no_command() {
        let removal = Removal::Files {
            paths: vec![PathBuf::from("/opt")],
            access: Access::Root,
        };
        assert_eq!(other_spec(&removal), None);
    }

    #[test]
    fn user_files_go_to_the_recycle_bin() {
        let removal = Removal::Files {
            paths: vec![PathBuf::from("/home/me/.local/bin/my tool")],
            access: Access::User,
        };
        let spec = other_spec(&removal);
        assert_eq!(
            spec.as_ref().map(|s| s.argv.clone()),
            Some(words(&["gio", "trash", "--", "/home/me/.local/bin/my tool"]))
        );
        assert_eq!(
            spec.map(|s| s.shown),
            Some("gio trash -- '/home/me/.local/bin/my tool'".to_owned())
        );
    }

    #[test]
    fn a_system_flatpak_asks_for_the_password() {
        let removal = Removal::Flatpak {
            app_id: "io.github.peazip.PeaZip".to_owned(),
            scope: FlatpakScope::System,
        };
        let spec = other_spec(&removal);
        assert_eq!(spec.as_ref().map(|s| s.argv[0].as_str()), Some("pkexec"));
        assert_eq!(
            spec.map(|s| s.shown),
            Some("sudo flatpak uninstall --noninteractive --system io.github.peazip.PeaZip".to_owned())
        );
    }

    #[test]
    fn a_flatpak_goes_from_where_it_was_installed() {
        let removal = Removal::Flatpak {
            app_id: "org.app".to_owned(),
            scope: FlatpakScope::User,
        };
        assert_eq!(
            other_spec(&removal).map(|s| s.shown),
            Some("flatpak uninstall --noninteractive --user org.app".to_owned())
        );
    }

    #[test]
    fn a_manager_runs_as_the_user_unless_the_package_is_in_the_system() {
        let argv = words(&["npm", "uninstall", "-g", "stray"]);
        let user = Removal::Manager {
            name: "npm".to_owned(),
            argv: argv.clone(),
            access: Access::User,
        };
        let root = Removal::Manager {
            name: "npm".to_owned(),
            argv: argv.clone(),
            access: Access::Root,
        };
        assert_eq!(other_spec(&user).map(|s| s.argv), Some(argv));
        let spec = other_spec(&root);
        assert_eq!(spec.as_ref().map(|s| s.argv[0].as_str()), Some("pkexec"));
        assert_eq!(
            spec.map(|s| s.shown),
            Some("sudo npm uninstall -g stray".to_owned())
        );
    }

    #[test]
    fn docker_removes_each_object_by_its_own_command() {
        let command = |object, id: &str| {
            other_spec(&Removal::Docker {
                object,
                id: id.to_owned(),
            })
            .map(|spec| spec.shown)
        };
        assert_eq!(
            command(DockerObject::Container, "becf455de171"),
            Some("docker container rm becf455de171".to_owned())
        );
        assert_eq!(
            command(DockerObject::Image, "sha256:0b0e"),
            Some("docker image rm sha256:0b0e".to_owned())
        );
        assert_eq!(
            command(DockerObject::Volume, "server_data"),
            Some("docker volume rm server_data".to_owned())
        );
    }

    #[test]
    fn what_is_unavailable_has_no_command() {
        let removal = Removal::Unavailable {
            reason: "x".to_owned(),
        };
        assert_eq!(other_spec(&removal), None);
    }

    #[test]
    fn a_dismissed_password_window_is_not_a_failure() {
        assert_eq!(outcome_of(Some(126), "pkexec"), Outcome::Dismissed);
        assert_eq!(outcome_of(Some(0), "pkexec"), Outcome::Done);
        assert_eq!(
            outcome_of(Some(126), "cargo"),
            Outcome::Failed(Failure::Code(126))
        );
    }

    #[test]
    fn output_of_both_streams_arrives_before_the_end() {
        let spec = Spec {
            argv: words(&["sh", "-c", "echo out; echo err >&2; exit 3"]),
            shown: String::new(),
        };
        let events: Vec<Event> = run_streaming(&spec).iter().collect();
        assert!(events.contains(&Event::Line("out".to_owned())));
        assert!(events.contains(&Event::Line("err".to_owned())));
        assert_eq!(
            events.last(),
            Some(&Event::Finished(Outcome::Failed(Failure::Code(3))))
        );
    }
}
