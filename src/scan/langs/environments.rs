//! Python environments of virtualenvwrapper, pyenv, conda, mamba and uv.

use std::fs;
use std::path::Path;

use super::{Scan, file_name, manager, tool, trash_item};
use crate::i18n::Text;
use crate::model::{Access, Source};
use crate::scan::{Places, blank, loose};

/// Isolated environments, one folder per tool, as `pipx` and `uv tool` keep them.
pub(super) fn environment_items(places: &Places, text: &Text, out: &mut Scan) {
    let kinds = [
        (".local/share/pipx/venvs", "pipx", ["uninstall"].as_slice()),
        (".local/share/uv/tools", "uv", ["tool", "uninstall"].as_slice()),
    ];
    for (folder, program, verb) in kinds {
        let root = places.home.join(folder);
        for environment in loose::directories_in(&root) {
            let name = environment
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut argv = vec![tool(places, program)];
            argv.extend(verb.iter().map(|word| (*word).to_owned()));
            argv.push(name.clone());
            let id = format!("{program}:{name}");
            let mut item = blank(
                id,
                name.clone(),
                Source::Python,
                manager(program, argv, Access::User),
            );
            item.summary = (text.python_tool)(program);
            item.description = item.summary.clone();
            item.homepage = Some(format!("https://pypi.org/project/{name}"));
            item.size = Some(loose::size_of(&environment));
            out.items.push(item);
        }
        out.claimed.push(root);
    }
}

/// The Python version of a virtual environment, from its `pyvenv.cfg`.
pub(super) fn pyvenv_version(environment: &Path) -> Option<String> {
    let config = fs::read_to_string(environment.join("pyvenv.cfg")).ok()?;
    config.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        matches!(key.trim(), "version" | "version_info")
            .then(|| value.trim().split('.').take(3).collect::<Vec<_>>().join("."))
    })
}

/// The Python version of a conda environment: the name of the record of its `python` package.
pub(super) fn conda_version(environment: &Path) -> Option<String> {
    loose::entries_in(&environment.join("conda-meta"))
        .iter()
        .filter_map(|record| record.file_name()?.to_str()?.strip_prefix("python-"))
        .find(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        .and_then(|rest| rest.split('-').next())
        .map(str::to_owned)
}

/// pyenv keeps interpreters in `versions`, and a pyenv-virtualenv environment inside its
/// interpreter with a link to it next to them.
pub(super) fn pyenv_items(places: &Places, text: &Text, out: &mut Scan) {
    let versions = places.home.join(".pyenv/versions");
    for entry in loose::entries_in(&versions) {
        let name = file_name(&entry);
        if let Ok(target) = fs::read_link(&entry) {
            let target = versions.join(target);
            if target.join("pyvenv.cfg").is_file() {
                let version = pyvenv_version(&target);
                let summary = (text.python_environment)("pyenv-virtualenv");
                out.items
                    .push(trash_item(vec![entry, target], &name, version, summary));
            }
        } else if entry.is_dir() {
            let mut item = trash_item(
                vec![entry],
                &name,
                Some(name.clone()),
                (text.python_interpreter)("pyenv"),
            );
            item.title = format!("Python {name} (pyenv)");
            out.items.push(item);
        }
    }
    out.claimed.push(versions);
}

/// Environments kept where their tools put them: virtualenvwrapper, pyenv, conda and mamba.
pub(super) fn known_environments(places: &Places, text: &Text, out: &mut Scan) {
    let home = &places.home;
    let virtualenvs = home.join(".virtualenvs");
    for environment in loose::directories_in(&virtualenvs) {
        if environment.join("pyvenv.cfg").is_file() {
            let version = pyvenv_version(&environment);
            let summary = (text.python_environment)("virtualenvwrapper");
            out.items.push(trash_item(
                vec![environment.clone()],
                &file_name(&environment),
                version,
                summary,
            ));
        }
    }
    out.claimed.push(virtualenvs);
    pyenv_items(places, text, out);
    let conda = [
        ("miniconda3", "conda"),
        ("anaconda3", "conda"),
        ("miniforge3", "conda"),
        ("mambaforge", "mamba"),
        (".conda", "conda"),
        ("micromamba", "mamba"),
        (".local/share/mamba", "mamba"),
    ];
    for (root, tool) in conda {
        let envs = home.join(root).join("envs");
        for environment in loose::directories_in(&envs) {
            if environment.join("conda-meta").is_dir() {
                let version = conda_version(&environment);
                let summary = (text.python_environment)(tool);
                out.items.push(trash_item(
                    vec![environment.clone()],
                    &file_name(&environment),
                    version,
                    summary,
                ));
            }
        }
        out.claimed.push(envs);
    }
}

pub(super) fn uv_pythons(places: &Places, text: &Text, out: &mut Scan) {
    let root = places.home.join(".local/share/uv/python");
    for folder in loose::directories_in(&root) {
        let is_link = fs::symlink_metadata(&folder).is_ok_and(|meta| meta.file_type().is_symlink());
        let key = folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if is_link || !key.starts_with("cpython-") && !key.starts_with("pypy-") {
            continue;
        }
        let mut parts = key.split('-');
        let implementation = parts.next().unwrap_or_default();
        let version = parts.next().unwrap_or_default().to_owned();
        let argv = vec![
            tool(places, "uv"),
            "python".to_owned(),
            "uninstall".to_owned(),
            key.clone(),
        ];
        let id = format!("uv-python:{key}");
        let mut item = blank(id, key.clone(), Source::Python, manager("uv", argv, Access::User));
        item.title = format!("Python {version} ({implementation})");
        item.version = version;
        item.summary = (text.python_interpreter)("uv");
        item.description = item.summary.clone();
        item.homepage = Some("https://www.python.org".to_owned());
        item.size = Some(loose::size_of(&folder));
        out.items.push(item);
    }
    out.claimed.push(root);
}
