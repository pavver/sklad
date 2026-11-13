//! What Docker keeps: containers, images and volumes, read from `docker system df -v`.
//!
//! One call gives all three with their sizes. Without Docker, or without the right to reach its
//! daemon, the call prints nothing and the catalogue simply has no Docker entries.

use serde_json::Value;

use super::blank;
use crate::i18n::Text;
use crate::model::{DockerObject, Item, Removal, Source, Usage};

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
    container.image == image.name
        || format!("{}:latest", container.image) == image.name
        || image
            .id
            .trim_start_matches("sha256:")
            .starts_with(&container.image)
}

fn docker_item(object: DockerObject, id: &str, name: &str) -> Item {
    let key = match object {
        DockerObject::Container => "docker-container",
        DockerObject::Image => "docker-image",
        DockerObject::Volume => "docker-volume",
    };
    let removal = Removal::Docker {
        object,
        id: id.to_owned(),
    };
    blank(format!("{key}:{id}"), name.to_owned(), Source::Docker, removal)
}

pub fn items(docker: &Docker, text: &Text) -> Vec<Item> {
    let mut items = Vec::new();
    for container in &docker.containers {
        let mut item = docker_item(DockerObject::Container, &container.id, &container.name);
        item.version = container.image.clone();
        item.summary = (text.docker_container)(&container.image);
        item.description = item.summary.clone();
        item.size = container.size;
        item.installed = container.created;
        item.groups = container.project.iter().cloned().collect();
        item.depends = std::iter::once(container.image.clone())
            .chain(container.volumes.iter().cloned())
            .collect();
        if container.running {
            item.usage = Usage::Running;
        }
        items.push(item);
    }
    for image in &docker.images {
        let mut item = docker_item(DockerObject::Image, &image.id, &image.name);
        item.version = image.tag.clone();
        item.summary = text.docker_image.to_owned();
        item.description = item.summary.clone();
        item.size = image.size;
        item.installed = image.created;
        item.required_by = docker
            .containers
            .iter()
            .filter(|container| runs(container, image))
            .map(|container| container.name.clone())
            .collect();
        items.push(item);
    }
    for volume in &docker.volumes {
        let mut item = docker_item(DockerObject::Volume, &volume.name, &volume.name);
        item.summary = text.docker_volume.to_owned();
        item.description = item.summary.clone();
        item.size = volume.size;
        item.required_by = docker
            .containers
            .iter()
            .filter(|container| container.volumes.contains(&volume.name))
            .map(|container| container.name.clone())
            .collect();
        items.push(item);
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::remove::is_irreversible;

    const DF: &str = r#"{
      "Images": [
        {"ID": "sha256:0b0ea86881537b3f645aace7e8e3", "Repository": "rust", "Tag": "latest",
         "Size": "1.33GB", "CreatedAt": "2026-11-12 22:25:20 +0200 EET"},
        {"ID": "sha256:dea68412bf5c099f4eb8a6a2f63a", "Repository": "<none>", "Tag": "<none>",
         "Size": "52.1MB", "CreatedAt": "2026-10-01 10:00:00 +0000 UTC"}
      ],
      "Containers": [
        {"ID": "becf455de171", "Names": "server-remote-1", "Image": "rust", "State": "running",
         "Size": "1.02kB (virtual 1.33GB)", "CreatedAt": "2026-10-30 00:07:22 +0200 EET",
         "Labels": "com.docker.compose.oneoff=False,com.docker.compose.project=server",
         "Mounts": "server_data"},
        {"ID": "a1b2c3", "Names": "old", "Image": "dea68412bf5c", "State": "exited",
         "Size": "0B", "CreatedAt": "2026-10-02 10:00:00 +0000 UTC", "Labels": "", "Mounts": ""}
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

    #[test]
    fn each_object_is_removed_by_its_own_command_and_knows_its_users() {
        let items = items(&parse(DF), Lang::En.text());
        let find = |id: &str| items.iter().find(|item| item.id == id);
        let server = find("docker-container:becf455de171");
        assert_eq!(server.map(|item| item.usage), Some(Usage::Running));
        assert_eq!(
            server.map(|item| item.depends.clone()),
            Some(vec!["rust".to_owned(), "server_data".to_owned()])
        );
        let rust = find("docker-image:sha256:0b0ea86881537b3f645aace7e8e3");
        assert_eq!(
            rust.map(|item| item.required_by.clone()),
            Some(vec!["server-remote-1".to_owned()])
        );
        let untagged = find("docker-image:sha256:dea68412bf5c099f4eb8a6a2f63a");
        assert_eq!(
            untagged.map(|item| item.required_by.clone()),
            Some(vec!["old".to_owned()])
        );
        let data = find("docker-volume:server_data");
        assert_eq!(
            data.map(|item| item.removal.clone()),
            Some(Removal::Docker {
                object: DockerObject::Volume,
                id: "server_data".to_owned()
            })
        );
        assert_eq!(data.and_then(|item| item.size), Some(284_500_000));
        assert!(items.iter().all(|item| item.source == Source::Docker));
        let lost: Vec<bool> = [server, rust, data]
            .map(|item| item.is_some_and(is_irreversible))
            .to_vec();
        assert_eq!(lost, [true, false, true], "an image can be pulled again");
    }
}
