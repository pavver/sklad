//! The French catalogue.

use super::{DateStyle, Text};
use crate::model::{Section, SourceKind};

/// French counts take the singular for zero and one: 0 jour, 1 jour, 2 jours.
fn counted(count: i64, one: &str, many: &str) -> String {
    let noun = if count <= 1 { one } else { many };
    format!("{count} {noun}")
}

fn section(section: Section) -> &'static str {
    match section {
        Section::Internet => "Internet",
        Section::Multimedia => "Multimédia",
        Section::Graphics => "Graphisme",
        Section::Office => "Bureautique",
        Section::Development => "Développement",
        Section::Games => "Jeux",
        Section::Education => "Éducation",
        Section::Science => "Sciences",
        Section::System => "Système",
        Section::Utilities => "Utilitaires",
        Section::Console => "Console",
        Section::Libraries => "Bibliothèques",
        Section::Fonts => "Polices",
        Section::Drivers => "Noyau et pilotes",
        Section::Meta => "Métapaquets",
    }
}

fn source(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Repo => "Dépôts",
        SourceKind::Aur => "AUR",
        SourceKind::Flatpak => "Flatpak",
        SourceKind::Steam => "Steam",
        SourceKind::Cargo => "Cargo",
        SourceKind::Node => "Node.js",
        SourceKind::Python => "Python",
        SourceKind::Rustup => "Rustup",
        SourceKind::LocalDeb => "Paquets locaux",
        SourceKind::Docker => "Docker",
        SourceKind::Manual => "Hors gestionnaires",
    }
}

pub static TEXT: Text = Text {
    app_name: "Sklad",
    language: "Langue",
    no_matches: "Aucun résultat",

    reading_system: "Lecture du système…",
    nothing_found: "Rien trouvé. Modifiez la recherche ou les filtres.",
    dark_theme: "Thème sombre",
    refresh: "Actualiser",
    search: "Rechercher",
    clear_search: "Effacer la recherche",
    removed: |title| format!("Supprimé : {title}"),
    records: |count| counted(count as i64, "entrée", "entrées"),

    sort_by: "Trier par :",
    sort_last_run: "Dernier lancement",
    sort_name: "Nom",
    sort_size: "Taille",
    sort_installed: "Installé",
    ascending: "Croissant",
    descending: "Décroissant",
    origin_all: "Tout",
    origin_explicit: "Installés par vous",
    origin_dependency: "Dépendances",
    origin_orphans: "Inutiles",

    all_programs: "Tous les programmes",
    other: "Autres",
    everything: "Tout ce qui est installé",
    source_heading: "Source",
    all_sources: "Toutes les sources",
    section,
    source,

    column_program: "Programme",
    column_size: "Taille",
    column_last_run: "Dernier lancement",

    badge_orphan: "Dépendance inutile",
    badge_running: "En cours",
    key_version: "Version",
    key_package: "Paquet",
    key_license: "Licence",
    key_developer: "Développeur",
    key_size: "Taille",
    key_installed: "Installé",
    key_reason: "Arrivé comment",
    key_launch: "Dernier lancement",
    key_groups: "Groupes",
    key_removal: "Suppression",
    reason_explicit: "Vous l'avez installé",
    reason_dependency: "Comme dépendance d'un autre paquet",
    heading_description: "Description",
    heading_launchers: "Lanceurs",
    heading_command: "Commande de suppression",
    needed_by_others: "D'autres programmes ont besoin de ce paquet. Les options sont dans la fenêtre de suppression.",
    required_by: |count| format!("Requis par ({count})"),
    depends_on: |count| format!("Dépend de ({count})"),
    and_more: |count| format!("et {count} de plus"),
    remove: "Supprimer",
    launch: "Lancer",
    launch_failed: |title| format!("Impossible de lancer « {title} »"),
    site: "Site web",
    copy: "Copier",
    by_package: |manager| format!("Par le gestionnaire de paquets {manager}"),
    by_steam: "Par le client Steam",
    by_language_manager: "Par le gestionnaire de paquets de son langage",
    by_flatpak: "Par Flatpak",
    by_cargo: "Par cargo",
    not_removable_here: "Impossible à supprimer d'ici",
    files_to_bin: "Les fichiers vont à la corbeille",
    files_for_good: "Les fichiers sont supprimés définitivement, un mot de passe est nécessaire",

    size_units: ["o", "Ko", "Mo", "Go", "To"],
    decimal_mark: ',',
    date_style: DateStyle::DayFirst,
    today: "aujourd'hui",
    yesterday: "hier",
    days_ago: |days| format!("il y a {}", counted(days, "jour", "jours")),
    months_ago: |months| format!("il y a {months} mois"),
    years_ago: |years| format!("il y a {}", counted(years, "an", "ans")),
    running_now: "en cours",
    no_record: "aucune trace",
    running_now_long: "En cours d'exécution",
    no_record_long: "Aucune trace. Plasma ne voit que les lancements depuis le menu, le panneau et KRunner.",
    not_tracked_long: "Les lancements de ce type d'entrée ne sont pas suivis",
    plasma_log: "le journal de Plasma",
    last_launch: |when, source| format!("{when}, d'après {source}"),

    confirm_title: |title| format!("Supprimer « {title} » ?"),
    other_list_refused: |manager| {
        format!(
            "{manager} supprimerait une autre liste de paquets, rien n'a donc été fait. Actualisez le catalogue et réessayez."
        )
    },
    manager_refused: |manager, message| format!("{manager} a refusé :\n{message}"),
    path_too_close: "Ce chemin est trop proche de la racine du système, la suppression est donc interdite.",
    no_password: "Aucun mot de passe saisi, rien n'a changé.",
    needed_by_programs: |names| format!("Ces programmes ont besoin de ce paquet : {names}"),
    keep_everything: "Tout laisser tel quel",
    remove_with_them: "Supprimer avec eux",
    held_by_meta: |metas, kept| {
        format!(
            "Le métapaquet {metas} retient ce programme. Il sera supprimé aussi, mais ce n'est qu'une enveloppe vide. Ses autres paquets restent dans le système et sont marqués comme installés par vous : {kept}."
        )
    },
    critical_packages: |names| format!("Le système risque de ne pas démarrer sans ces paquets : {names}"),
    plan_summary: |count, freed| {
        let packages = counted(count as i64, "paquet", "paquets");
        let verb = if count <= 1 {
            "sera supprimé"
        } else {
            "seront supprimés"
        };
        format!("{packages} {verb}, {freed} seront libérés. Les réglages du dossier personnel restent.")
    },
    through_manager: |name| format!("Sera supprimé par {name}."),
    flatpak_note: "Le programme sera supprimé par Flatpak. Ses données dans le dossier personnel restent.",
    steam_note: "Une fenêtre Steam va s'ouvrir. Confirmez-y la suppression, puis actualisez le catalogue.",
    cargo_note: "Le programme sera supprimé par cargo.",
    files_go_to_bin: "Les fichiers iront à la corbeille :",
    files_go_for_good: "Les fichiers seront supprimés définitivement :",
    not_a_package: "Ce n'est pas un paquet : les fichiers ne pourront être restaurés que depuis un instantané ou une sauvegarde.",
    checking: |manager| format!("Vérification avec {manager}…"),
    removing: "Suppression…",
    done: "Terminé",
    failed: |message| format!("Échec de la suppression : {message}. Voir la sortie ci-dessus."),
    close: "Fermer",
    cancel: "Annuler",
    copy_command: "Copier la commande",
    still_running: "Le programme est en cours d'exécution. Fermez-le avant de le supprimer.",
    understood: "Je comprends les conséquences",
    exit_code: |code| format!("code de sortie {code}"),
    interrupted: "le processus a été interrompu",
    empty_command: "commande vide",

    steam_component: "Composant Steam",
    steam_game: "Jeu Steam",
    programs: |names| format!("Programmes : {names}"),
    loose_file: |path| format!("Un fichier hors des gestionnaires de paquets : {path}"),
    loose_launcher: |path| format!("Un lanceur hors des gestionnaires de paquets : {path}"),
    stray_python: |path| {
        format!(
            "Cet enregistrement se trouve dans le Python du système hors du gestionnaire de paquets : {path}. Il peut rester d'une mise à jour, la suppression automatique est donc désactivée."
        )
    },
    python_tool: |manager| format!("Un outil Python dans son propre environnement, installé par {manager}"),
    python_interpreter: |manager| format!("Un interpréteur Python installé par {manager}"),
    python_environment: |tool| format!("Un environnement Python de {tool}"),
    by_docker: "Par Docker",
    docker_container: |image| format!("Un conteneur Docker créé à partir de {image}"),
    docker_image: "Une image Docker",
    docker_volume: "Un volume Docker : des données gardées pour les conteneurs",
    docker_data_lost: "Son contenu ne pourra pas être récupéré : Docker le supprime définitivement.",
    rust_toolchain: "Une chaîne d'outils Rust : le compilateur, cargo et la bibliothèque standard",
    default_toolchain: "C'est la chaîne d'outils par défaut : cargo et rustc cessent de fonctionner sans elle.",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_counts_keep_zero_and_one_singular() {
        assert_eq!((TEXT.days_ago)(1), "il y a 1 jour");
        assert_eq!((TEXT.days_ago)(3), "il y a 3 jours");
        assert_eq!((TEXT.records)(0), "0 entrée");
        assert_eq!(
            (TEXT.plan_summary)(2, "3,0 Mo"),
            "2 paquets seront supprimés, 3,0 Mo seront libérés. Les réglages du dossier personnel restent."
        );
    }
}
