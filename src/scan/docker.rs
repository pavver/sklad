//! What Docker keeps: containers, images and volumes, read from `docker system df -v`.
//!
//! One call gives all three with their sizes. Without Docker, or without the right to reach its
//! daemon, the call prints nothing and the catalogue simply has no Docker entries.
//!
//! The catalogue groups them the way a package groups its files: a Compose project, or a
//! container outside any project, is one row with the images and volumes only it uses. An image
//! or a volume more than one of them uses, or none does, keeps a row of its own.

use std::collections::BTreeMap;

use serde_json::Value;

use super::blank;
use crate::i18n::Text;
use crate::model::{DockerKind, DockerObject, DockerPart, Item, Reason, Removal, Source, Usage};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub running: bool,
    pub size: Option<u64>,
    pub created: Option<i64>,
    /// The Compose project that started it, when one did.
    pub project: Option<String>,
    pub volumes: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Image {
    pub id: String,
    /// `repository:tag`, or the short id for an image with neither.
    pub name: String,
    pub tag: String,
    pub size: Option<u64>,
    /// What only this image holds, which is what removing it frees; layers it shares with
    /// other images stay.
    pub unique_size: Option<u64>,
    pub created: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Volume {
    pub name: String,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Docker {
    pub containers: Vec<Container>,
    pub images: Vec<Image>,
    pub volumes: Vec<Volume>,
}

/// A size as Docker prints it, in powers of 1000: `1.33GB`, `1.02kB`, `0B`. A container size
/// may carry `(virtual …)` after it, which is the image and not the container.
pub fn parse_size(text: &str) -> Option<u64> {
    let text = text.split_whitespace().next()?;
    let digits = text
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(digits);
    let number: f64 = number.parse().ok()?;
    let scale = match unit {
        "B" | "" => 1.0,
        "kB" | "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        _ => return None,
    };
    Some((number * scale).round() as u64)
}

/// Days since 1970-01-01 of a civil date, after Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

/// A time as Docker prints it, `2026-10-30 00:07:22 +0200 EET`, in Unix seconds.
pub fn parse_time(text: &str) -> Option<i64> {
    let mut parts = text.split_whitespace();
    let mut date = parts.next()?.split('-').map(str::parse::<i64>);
    let mut clock = parts.next()?.split(':').map(str::parse::<i64>);
    let offset = parts.next()?;
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let (hour, minute, second) = (clock.next()?.ok()?, clock.next()?.ok()?, clock.next()?.ok()?);
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let offset = offset.trim_start_matches(['+', '-']);
    let offset_hours: i64 = offset.get(..2)?.parse().ok()?;
    let offset_minutes: i64 = offset.get(2..4)?.parse().ok()?;
    let local = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    Some(local - sign * (offset_hours * 3600 + offset_minutes * 60))
}

fn field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The value of one `key=value` label in Docker's comma-joined list.
fn label(labels: &str, key: &str) -> Option<String> {
    labels
        .split(',')
        .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn list(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn container(value: &Value) -> Container {
    Container {
        id: field(value, "ID"),
        name: field(value, "Names"),
        image: field(value, "Image"),
        running: field(value, "State") == "running",
        size: parse_size(&field(value, "Size")),
        created: parse_time(&field(value, "CreatedAt")),
        project: label(&field(value, "Labels"), "com.docker.compose.project"),
        volumes: field(value, "Mounts")
            .split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}

fn image(value: &Value) -> Image {
    let id = field(value, "ID");
    let repository = field(value, "Repository");
    let tag = field(value, "Tag");
    let name = if repository.is_empty() || repository == "<none>" {
        id.trim_start_matches("sha256:").chars().take(12).collect()
    } else if tag.is_empty() || tag == "<none>" {
        repository
    } else {
        format!("{repository}:{tag}")
    };
    Image {
        id,
        name,
        tag: if tag == "<none>" { String::new() } else { tag },
        size: parse_size(&field(value, "Size")),
        unique_size: parse_size(&field(value, "UniqueSize")),
        created: parse_time(&field(value, "CreatedAt")),
    }
}

/// The output of `docker system df -v --format '{{json .}}'`.
pub fn parse(json: &str) -> Docker {
    let Ok(value) = serde_json::from_str::<Value>(json) else {
        return Docker::default();
    };
    Docker {
        containers: list(&value, "Containers").iter().map(container).collect(),
        images: list(&value, "Images").iter().map(image).collect(),
        volumes: list(&value, "Volumes")
            .iter()
            .map(|volume| Volume {
                name: field(volume, "Name"),
                size: parse_size(&field(volume, "Size")),
            })
            .collect(),
    }
}

/// Whether a container runs this image: Docker names it as it was asked for, so `rust` stands
/// for `rust:latest`.
fn runs(container: &Container, image: &Image) -> bool {
    !container.image.is_empty() && container.image == image.name
        || format!("{}:latest", container.image) == image.name
        || image
            .id
            .trim_start_matches("sha256:")
            .starts_with(&container.image)
}

/// A Compose project, or a container outside any project.
struct Unit<'a> {
    project: Option<&'a str>,
    containers: Vec<&'a Container>,
}

impl Unit<'_> {
    fn title(&self) -> &str {
        match (self.project, self.containers.first()) {
            (Some(project), _) => project,
            (None, Some(container)) => &container.name,
            (None, None) => "",
        }
    }

    fn uses_image(&self, image: &Image) -> bool {
        self.containers.iter().any(|container| runs(container, image))
    }

    fn uses_volume(&self, volume: &Volume) -> bool {
        self.containers
            .iter()
            .any(|container| container.volumes.contains(&volume.name))
    }
}

fn units(docker: &Docker) -> Vec<Unit<'_>> {
    let mut projects: BTreeMap<&str, Vec<&Container>> = BTreeMap::new();
    let mut units = Vec::new();
    for container in &docker.containers {
        match &container.project {
            Some(project) => projects.entry(project).or_default().push(container),
            None => units.push(Unit {
                project: None,
                containers: vec![container],
            }),
        }
    }
    units.extend(projects.into_iter().map(|(project, containers)| Unit {
        project: Some(project),
        containers,
    }));
    units
}

fn image_part(image: &Image) -> DockerPart {
    DockerPart {
        object: DockerObject::Image,
        id: image.id.clone(),
        name: image.name.clone(),
        size: image.unique_size.or(image.size),
    }
}

fn volume_part(volume: &Volume) -> DockerPart {
    DockerPart {
        object: DockerObject::Volume,
        id: volume.name.clone(),
        name: volume.name.clone(),
        size: volume.size,
    }
}

/// The sum of the sizes Docker reported, `None` when it reported none.
fn total(parts: &[DockerPart]) -> Option<u64> {
    parts
        .iter()
        .filter_map(|part| part.size)
        .fold(None, |sum, size| Some(sum.unwrap_or(0) + size))
}

fn docker_item(kind: DockerKind, key: &str, name: &str) -> Item {
    let prefix = match kind {
        DockerKind::Project => "docker-project",
        DockerKind::Container => "docker-container",
        DockerKind::Image => "docker-image",
        DockerKind::Volume => "docker-volume",
    };
    let mut item = blank(
        format!("{prefix}:{key}"),
        name.to_owned(),
        Source::Docker,
        Removal::Docker { parts: Vec::new() },
    );
    item.docker = Some(kind);
    item
}

fn unit_item(unit: &Unit<'_>, own: Vec<DockerPart>, shared: Vec<String>, text: &Text) -> Item {
    let mut parts: Vec<DockerPart> = unit
        .containers
        .iter()
        .map(|container| DockerPart {
            object: DockerObject::Container,
            id: container.id.clone(),
            name: container.name.clone(),
            size: container.size,
        })
        .collect();
    parts.extend(own);
    let title = unit.title().to_owned();
    let mut item = match (unit.project, unit.containers.first()) {
        (None, Some(container)) => {
            let mut item = docker_item(DockerKind::Container, &container.id, &title);
            item.version = container.image.clone();
            item.summary = (text.docker_container)(&container.image);
            item
        }
        _ => {
            let mut item = docker_item(DockerKind::Project, &title, &title);
            item.summary = (text.docker_project)(unit.containers.len());
            item
        }
    };
    item.reason = Some(Reason::Explicit);
    item.size = total(&parts);
    item.installed = unit
        .containers
        .iter()
        .filter_map(|container| container.created)
        .min();
    item.depends = shared;
    if unit.containers.iter().any(|container| container.running) {
        item.usage = Usage::Running;
    }
    item.removal = Removal::Docker { parts };
    item
}

/// An image or a volume that is not one unit's own: nothing uses it, or several units do.
fn loose_item(kind: DockerKind, part: DockerPart, users: Vec<String>, summary: &str) -> Item {
    let mut item = docker_item(kind, &part.id, &part.name);
    item.reason = Some(Reason::Dependency);
    item.summary = summary.to_owned();
    item.size = part.size;
    item.required_by = users;
    item.removal = Removal::Docker { parts: vec![part] };
    item
}

pub fn items(docker: &Docker, text: &Text) -> Vec<Item> {
    let units = units(docker);
    let image_users: Vec<Vec<usize>> = docker
        .images
        .iter()
        .map(|image| {
            (0..units.len())
                .filter(|&at| units[at].uses_image(image))
                .collect()
        })
        .collect();
    let volume_users: Vec<Vec<usize>> = docker
        .volumes
        .iter()
        .map(|volume| {
            (0..units.len())
                .filter(|&at| units[at].uses_volume(volume))
                .collect()
        })
        .collect();
    let titles =
        |users: &[usize]| -> Vec<String> { users.iter().map(|&at| units[at].title().to_owned()).collect() };
    let mut own: Vec<Vec<DockerPart>> = vec![Vec::new(); units.len()];
    let mut shared: Vec<Vec<String>> = vec![Vec::new(); units.len()];
    let mut loose = Vec::new();
    for (image, users) in docker.images.iter().zip(&image_users) {
        if let [only] = users.as_slice() {
            own[*only].push(image_part(image));
            continue;
        }
        for &at in users {
            shared[at].push(image.name.clone());
        }
        let mut item = loose_item(
            DockerKind::Image,
            image_part(image),
            titles(users),
            text.docker_image,
        );
        item.version = image.tag.clone();
        item.installed = image.created;
        loose.push(item);
    }
    for (volume, users) in docker.volumes.iter().zip(&volume_users) {
        if let [only] = users.as_slice() {
            own[*only].push(volume_part(volume));
            continue;
        }
        for &at in users {
            shared[at].push(volume.name.clone());
        }
        loose.push(loose_item(
            DockerKind::Volume,
            volume_part(volume),
            titles(users),
            text.docker_volume,
        ));
    }
    let mut items: Vec<Item> = units
        .iter()
        .zip(own.into_iter().zip(shared))
        .map(|(unit, (own, shared))| unit_item(unit, own, shared, text))
        .chain(loose)
        .collect();
    for item in &mut items {
        item.description = item.summary.clone();
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::model::DockerKind;
    use crate::remove::is_irreversible;

    const DF: &str = r#"{
      "Images": [
        {"ID": "sha256:0b0ea86881537b3f645aace7e8e3", "Repository": "rust", "Tag": "latest",
         "Size": "1.33GB", "UniqueSize": "1.1GB", "CreatedAt": "2026-11-12 22:25:20 +0200 EET"},
        {"ID": "sha256:dea68412bf5c099f4eb8a6a2f63a", "Repository": "<none>", "Tag": "<none>",
         "Size": "52.1MB", "UniqueSize": "52.1MB", "CreatedAt": "2026-10-01 10:00:00 +0000 UTC"}
      ],
      "Containers": [
        {"ID": "becf455de171", "Names": "server-remote-1", "Image": "rust", "State": "running",
         "Size": "1.02kB (virtual 1.33GB)", "CreatedAt": "2026-10-30 00:07:22 +0200 EET",
         "Labels": "com.docker.compose.oneoff=False,com.docker.compose.project=server",
         "Mounts": "server_data"},
        {"ID": "a1b2c3", "Names": "old", "Image": "dea68412bf5c", "State": "exited",
         "Size": "0B", "CreatedAt": "2026-10-02 10:00:00 +0000 UTC", "Labels": "", "Mounts": ""},
        {"ID": "d4e5f6", "Names": "server-worker-1", "Image": "rust", "State": "exited",
         "Size": "2kB", "CreatedAt": "2026-10-29 10:00:00 +0000 UTC",
         "Labels": "com.docker.compose.project=server", "Mounts": "server_data"},
        {"ID": "0a0b0c", "Names": "scratch", "Image": "rust:latest", "State": "exited",
         "Size": "5kB", "CreatedAt": "2026-11-01 10:00:00 +0000 UTC", "Labels": "", "Mounts": ""}
      ],
      "Volumes": [
        {"Name": "server_data", "Size": "284.5MB"},
        {"Name": "cache", "Size": "N/A"}
      ],
      "BuildCache": []
    }"#;

    #[test]
    fn sizes_and_times_read_as_docker_prints_them() {
        assert_eq!(parse_size("1.33GB"), Some(1_330_000_000));
        assert_eq!(parse_size("1.02kB (virtual 1.33GB)"), Some(1020));
        assert_eq!(parse_size("0B"), Some(0));
        assert_eq!(parse_size("N/A"), None);
        assert_eq!(parse_time("1970-01-01 02:00:00 +0200 EET"), Some(0));
        assert_eq!(parse_time("2026-10-02 10:00:00 +0000 UTC"), Some(1_790_935_200));
        assert_eq!(parse_time("2026-10-02 05:30:00 -0430 VET"), Some(1_790_935_200));
        assert_eq!(parse_time("yesterday"), None);
    }

    #[test]
    fn containers_images_and_volumes_are_read() {
        let docker = parse(DF);
        let names = |list: Vec<&str>| list.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(
            docker.images.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["rust:latest", "dea68412bf5c"]
        );
        let server = docker.containers.first().cloned().unwrap_or_default();
        assert!(server.running);
        assert_eq!(server.size, Some(1020));
        assert_eq!(server.project.as_deref(), Some("server"));
        assert_eq!(server.volumes, names(vec!["server_data"]));
        assert_eq!(docker.volumes.get(1).and_then(|v| v.size), None);
        assert_eq!(parse("not json"), Docker::default());
    }

    fn part(object: DockerObject, id: &str, name: &str, size: Option<u64>) -> DockerPart {
        DockerPart {
            object,
            id: id.to_owned(),
            name: name.to_owned(),
            size,
        }
    }

    #[test]
    fn a_project_or_a_lone_container_is_one_row_with_what_only_it_uses() {
        let items = items(&parse(DF), Lang::En.text());
        let ids: Vec<&str> = items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "docker-container:a1b2c3",
                "docker-container:0a0b0c",
                "docker-project:server",
                "docker-image:sha256:0b0ea86881537b3f645aace7e8e3",
                "docker-volume:cache",
            ],
            "the untagged image and server_data belong to one row each"
        );
        let find = |id: &str| items.iter().find(|item| item.id == id);
        let server = find("docker-project:server");
        assert_eq!(server.and_then(|item| item.docker), Some(DockerKind::Project));
        assert_eq!(server.map(|item| item.usage), Some(Usage::Running));
        assert_eq!(
            server.map(|item| item.summary.as_str()),
            Some("A Docker Compose project: 2 containers")
        );
        assert_eq!(
            server.map(|item| item.removal.clone()),
            Some(Removal::Docker {
                parts: vec![
                    part(
                        DockerObject::Container,
                        "becf455de171",
                        "server-remote-1",
                        Some(1020)
                    ),
                    part(DockerObject::Container, "d4e5f6", "server-worker-1", Some(2000)),
                    part(
                        DockerObject::Volume,
                        "server_data",
                        "server_data",
                        Some(284_500_000)
                    ),
                ]
            })
        );
        assert_eq!(server.and_then(|item| item.size), Some(284_503_020));
        assert_eq!(
            server.map(|item| item.depends.clone()),
            Some(vec!["rust:latest".to_owned()])
        );

        let old = find("docker-container:a1b2c3");
        assert_eq!(old.and_then(|item| item.docker), Some(DockerKind::Container));
        assert_eq!(
            old.and_then(|item| item.size),
            Some(52_100_000),
            "the image only it uses is counted by what removing it frees"
        );

        let rust = find("docker-image:sha256:0b0ea86881537b3f645aace7e8e3");
        assert_eq!(
            rust.map(|item| item.required_by.clone()),
            Some(vec!["scratch".to_owned(), "server".to_owned()])
        );
        assert_eq!(rust.and_then(|item| item.size), Some(1_100_000_000));
        assert!(!rust.is_some_and(Item::is_orphan));
        let cache = find("docker-volume:cache");
        assert!(
            cache.is_some_and(Item::is_orphan),
            "a volume nothing uses is left over"
        );
        assert!(items.iter().all(|item| item.source == Source::Docker));
        let lost: Vec<bool> = [server, rust, cache]
            .map(|item| item.is_some_and(is_irreversible))
            .to_vec();
        assert_eq!(lost, [true, false, true], "an image can be pulled again");
    }
}
