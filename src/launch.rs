//! Starting a program from its card.

use crate::model::Launch;
use crate::remove::Spec;

/// The command that starts the program as the menu would, for the person, without a password.
pub fn spec(launch: &Launch) -> Spec {
    let argv = match launch {
        Launch::Desktop(path) => vec!["gio".to_owned(), "launch".to_owned(), path.display().to_string()],
        Launch::Steam { app_id } => vec!["xdg-open".to_owned(), format!("steam://rungameid/{app_id}")],
    };
    let shown = argv.join(" ");
    Spec { argv, shown }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn a_launcher_file_is_started_through_gio() {
        let launch = Launch::Desktop(PathBuf::from("/usr/share/applications/org.kde.okular.desktop"));
        assert_eq!(
            spec(&launch).argv,
            ["gio", "launch", "/usr/share/applications/org.kde.okular.desktop"]
        );
    }

    #[test]
    fn a_steam_game_is_started_by_the_steam_client() {
        let launch = Launch::Steam {
            app_id: "620".to_owned(),
        };
        assert_eq!(spec(&launch).argv, ["xdg-open", "steam://rungameid/620"]);
    }
}
