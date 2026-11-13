//! Programs installed by hand: files in the folders of programs that no package owns.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::i18n::Text;
use crate::model::{Access, Item, Removal, Section, Source};

use super::appstream::Metainfo;
use super::desktop::Launcher;
use super::{Context, blank, desktop, dress, loose, usage_of};

fn manual_file(path: &Path, access: Access, text: &Text) -> Item {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let removal = Removal::Files {
        paths: vec![path.to_path_buf()],
        access,
    };
    let mut item = blank(format!("file:{}", path.display()), name, Source::Manual, removal);
    item.summary = path.display().to_string();
    item.description = (text.loose_file)(&path.display().to_string());
    item.size = Some(loose::size_of(path));
    item
}

/// Loose files, then user launchers: a launcher that starts one of those files joins it, so the
/// person sees one program, and removing it takes both.
pub(super) fn manual_items(
    claimed: &HashSet<PathBuf>,
    claimed_folders: &[PathBuf],
    owned_programs: &HashMap<String, String>,
    owned_opt: &HashSet<String>,
    context: &Context<'_>,
) -> ManualScan {
    let (mut items, by_program) = user_files(claimed, claimed_folders, context);
    items.extend(system_files(owned_opt, context));
    let mut attached: HashMap<String, Vec<Launcher>> = HashMap::new();
    let folder = context.places.home.join(".local/share/applications");
    for path in loose::launchers_below(&folder) {
        let Some(launcher) = desktop::read(&path, context.lang) else {
            continue;
        };
        let exec = launcher.exec.clone().unwrap_or_default();
        if !launcher.shown || exec.contains("steam://") {
            continue;
        }
        let program = desktop::exec_program(&exec).unwrap_or_default();
        match owner_of(&program, owned_programs) {
            Some(owner) => attached.entry(owner.clone()).or_default().push(launcher),
            None => place_launcher(&mut items, &launcher, &path, &program, &by_program, context),
        }
    }
    ManualScan { items, attached }
}

/// Programs in the bin folders of the home that nothing else claims, and their file names.
fn user_files(
    claimed: &HashSet<PathBuf>,
    claimed_folders: &[PathBuf],
    context: &Context<'_>,
) -> (Vec<Item>, HashMap<String, PathBuf>) {
    let home = &context.places.home;
    let folders = [".local/bin", ".cargo/bin", "go/bin", ".deno/bin"].map(|folder| home.join(folder));
    let mut items = Vec::new();
    let mut by_program = HashMap::new();
    for path in folders.iter().flat_map(|dir| loose::files_in(dir)) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        let in_a_folder = claimed_folders.iter().any(|folder| canonical.starts_with(folder));
        if claimed.contains(&path) || in_a_folder {
            continue;
        }
        by_program.extend(file_name(&path).map(|name| (name, path.clone())));
        items.push(manual_file(&path, Access::User, context.lang.text()));
    }
    (items, by_program)
}

/// Programs in `/usr/local/bin`, and folders in `/opt` that no package owns.
fn system_files(owned_opt: &HashSet<String>, context: &Context<'_>) -> Vec<Item> {
    let text = context.lang.text();
    let programs = loose::files_in(&context.places.system_path("usr/local/bin"))
        .into_iter()
        .map(|path| manual_file(&path, Access::Root, text));
    let folders = loose::directories_in(&context.places.system_path("opt"))
        .into_iter()
        .filter(|path| !file_name(path).is_some_and(|name| owned_opt.contains(&name)))
        .map(|path| Item {
            section: Section::Utilities,
            ..manual_file(&path, Access::Root, text)
        });
    programs.chain(folders).collect()
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name().map(|name| name.to_string_lossy().into_owned())
}

/// The package that owns the program a launcher starts.
fn owner_of<'a>(program: &str, owned_programs: &'a HashMap<String, String>) -> Option<&'a String> {
    file_name(Path::new(program)).and_then(|name| owned_programs.get(&name))
}

/// A launcher of no package joins the loose file it starts, or becomes an entry of its own.
fn place_launcher(
    items: &mut Vec<Item>,
    launcher: &Launcher,
    path: &Path,
    program: &str,
    by_program: &HashMap<String, PathBuf>,
    context: &Context<'_>,
) {
    let target = loose::find_program(program, by_program).map(|file| format!("file:{}", file.display()));
    if let Some(item) = target.and_then(|id| items.iter_mut().find(|item| item.id == id)) {
        join_launcher(item, launcher, path, context);
        return;
    }
    let mut item = manual_file(path, Access::User, context.lang.text());
    item.description = (context.lang.text().loose_launcher)(&path.display().to_string());
    join_launcher(&mut item, launcher, path, context);
    items.push(item);
}

pub(super) struct ManualScan {
    pub(super) items: Vec<Item>,
    pub(super) attached: HashMap<String, Vec<Launcher>>,
}

fn join_launcher(item: &mut Item, launcher: &Launcher, path: &Path, context: &Context<'_>) {
    let list = std::slice::from_ref(launcher);
    dress(item, list, &Metainfo::default(), context.icons);
    item.usage = usage_of(list, context.records);
    if let Removal::Files { paths, .. } = &mut item.removal
        && !paths.iter().any(|known| known == path)
    {
        paths.push(path.to_path_buf());
    }
}
