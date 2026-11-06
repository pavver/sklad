//! Launch records. Plasma logs what was started through its menu, task bar and KRunner.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use rusqlite::{Connection, OpenFlags};

const QUERY: &str = "
    SELECT targettedResource, MAX(lastUpdate) FROM ResourceScoreCache
    WHERE targettedResource LIKE 'applications:%' GROUP BY targettedResource
    UNION ALL
    SELECT targettedResource, MAX(MAX(start), MAX(COALESCE(end, 0))) FROM ResourceEvent
    WHERE targettedResource LIKE 'applications:%' GROUP BY targettedResource";

/// The key a launcher is recorded under, in the form both sides can be compared in.
pub fn key(resource: &str) -> String {
    let name = resource.strip_prefix("applications:").unwrap_or(resource);
    name.strip_suffix(".desktop").unwrap_or(name).to_lowercase()
}

fn read(connection: &Connection) -> rusqlite::Result<HashMap<String, i64>> {
    let mut statement = connection.prepare(QUERY)?;
    let rows = statement.query_map([], |row| {
        let resource: String = row.get(0)?;
        let time: Option<i64> = row.get(1)?;
        Ok((key(&resource), time.unwrap_or(0)))
    })?;
    let mut launches: HashMap<String, i64> = HashMap::new();
    for row in rows {
        let (name, time) = row?;
        let latest = launches.entry(name).or_default();
        *latest = (*latest).max(time);
    }
    Ok(launches)
}

/// Last launch per launcher. Opened read-only: the log belongs to a running service. The second
/// attempt ignores the write-ahead log, for the case when its lock cannot be taken.
pub fn plasma_launches(database: &Path) -> HashMap<String, i64> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    let path = database.display();
    [
        format!("file:{path}?mode=ro"),
        format!("file:{path}?mode=ro&immutable=1"),
    ]
    .iter()
    .find_map(|uri| {
        let connection = Connection::open_with_flags(uri, flags).ok()?;
        read(&connection).ok()
    })
    .unwrap_or_default()
}

/// Paths of the programs alive right now.
pub fn running_programs() -> HashSet<String> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return HashSet::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(|c: char| c.is_ascii_digit())
        })
        .filter_map(|entry| fs::read_link(entry.path().join("exe")).ok())
        .map(|path| path.to_string_lossy().trim_end_matches(" (deleted)").to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_of_the_comparison_end_up_the_same() {
        assert_eq!(key("applications:org.kde.Dolphin.desktop"), "org.kde.dolphin");
        assert_eq!(key("org.kde.dolphin"), "org.kde.dolphin");
    }

    #[test]
    fn the_latest_record_of_both_tables_wins() -> rusqlite::Result<()> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(
            "CREATE TABLE ResourceScoreCache (targettedResource TEXT, lastUpdate INTEGER);
             CREATE TABLE ResourceEvent (targettedResource TEXT, start INTEGER, end INTEGER);
             INSERT INTO ResourceScoreCache VALUES ('applications:a.desktop', 100),
                                                   ('applications:b.desktop', 500),
                                                   ('file:///x', 900);
             INSERT INTO ResourceEvent VALUES ('applications:a.desktop', 200, 300),
                                              ('applications:b.desktop', 50, NULL);",
        )?;
        let launches = read(&connection)?;
        assert_eq!(launches.get("a"), Some(&300));
        assert_eq!(launches.get("b"), Some(&500));
        assert_eq!(launches.len(), 2);
        Ok(())
    }
}
