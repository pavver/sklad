//! Flatpak applications and runtimes.

use crate::model::FlatpakScope;

pub const COLUMNS: &str = "application,name,description,version,origin,installation,size,options";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub app_id: String,
    pub name: String,
    pub summary: String,
    pub version: String,
    pub scope: FlatpakScope,
    pub size: Option<u64>,
    pub runtime: bool,
}

/// `423.6 MB` -> bytes. Flatpak prints decimal units and may separate them with a no-break space.
pub fn parse_size(text: &str) -> Option<u64> {
    let clean = text.replace('\u{a0}', " ").replace(',', ".");
    let mut words = clean.split_whitespace();
    let number: f64 = words.next()?.parse().ok()?;
    let scale = match words.next().unwrap_or("B") {
        "B" | "bytes" => 1.0,
        "kB" | "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        _ => return None,
    };
    Some((number * scale) as u64)
}

pub fn parse(listing: &str) -> Vec<Entry> {
    listing
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let &[app_id, name, summary, version, _origin, install, size, options] = fields.as_slice() else {
                return None;
            };
            Some(Entry {
                app_id: app_id.to_owned(),
                name: name.to_owned(),
                summary: summary.to_owned(),
                version: version.to_owned(),
                scope: match install {
                    "user" => FlatpakScope::User,
                    _ => FlatpakScope::System,
                },
                size: parse_size(size),
                runtime: options.split(',').any(|option| option.trim() == "runtime"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listing_is_read() {
        let line = "org.app\tApp\tDoes things\t2.4\tflathub\tuser\t423.6\u{a0}MB\tcurrent\n\
            org.rt\tRuntime\t\t24\tflathub\tsystem\t1.0 GB\truntime,current\n";
        let entries = parse(line);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].scope, FlatpakScope::User);
        assert_eq!(entries[0].size, Some(423_600_000));
        assert!(!entries[0].runtime && entries[1].runtime);
    }
}
