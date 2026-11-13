//! Every word of the website, in English, Ukrainian, French and Russian.

/// The commands are the same in every language.
pub const TRUST_KEY: &str = "curl -sL https://pavver.github.io/sklad/sklad.asc | sudo pacman-key --add -
sudo pacman-key --lsign-key EB0FE0C7F0FEBBD1A40F8F93A7A3ED14D1C98D8D";
/// Appends the repository to pacman.conf unless it is already there.
pub const ADD_REPOSITORY: &str = r"grep -q '^\[sklad\]' /etc/pacman.conf ||
  printf '\n[sklad]\nServer = https://pavver.github.io/sklad/$arch\n' | sudo tee -a /etc/pacman.conf >/dev/null";
pub const INSTALL: &str = "sudo pacman -Sy sklad";
/// Trusts the key and adds the apt repository; running it again changes nothing.
pub const APT_ADD_REPOSITORY: &str = "curl -fsSL https://pavver.github.io/sklad/sklad.asc | sudo gpg --dearmor --yes -o /usr/share/keyrings/sklad.gpg
echo 'deb [signed-by=/usr/share/keyrings/sklad.gpg] https://pavver.github.io/sklad/debian ./' |
  sudo tee /etc/apt/sources.list.d/sklad.list >/dev/null";
pub const APT_INSTALL: &str = "sudo apt update && sudo apt install sklad";
pub const FROM_SOURCE: &str = "cargo install --git https://github.com/pavver/sklad";

/// Each page in its language: the name in that language, then in English, so a search finds it
/// either way. English needs no second name.
pub const PAGES: [(&str, &str); 4] = [
    ("Українська (Ukrainian)", "uk.html"),
    ("English", "./"),
    ("Français (French)", "fr.html"),
    ("Русский (Russian)", "ru.html"),
];

pub const REPOSITORY: &str = "https://github.com/pavver/sklad";
pub const DESIGN_SYSTEM: &str = "https://github.com/pavver/operator-design-system";

/// A question and its answer.
pub struct Question {
    pub title: &'static str,
    pub answer: &'static str,
}

/// A feature: its name and what it means.
pub struct Feature {
    pub title: &'static str,
    pub text: &'static str,
}

pub struct Text {
    pub name: &'static str,
    pub tagline: &'static str,
    /// The name of this page in `PAGES`.
    pub language_name: &'static str,
    pub dark_mode: &'static str,
    pub headline: &'static str,
    pub lead: &'static str,
    pub install_action: &'static str,
    pub source_action: &'static str,
    pub catalogue_shot: &'static str,
    pub catalogue_alt: &'static str,
    pub early_title: &'static str,
    pub early_text: &'static str,
    pub features_title: &'static str,
    pub features: [Feature; 7],
    pub removal_title: &'static str,
    pub removal_shot: &'static str,
    pub removal_alt: &'static str,
    pub removal_text: &'static str,
    pub install_title: &'static str,
    pub arch_tab: &'static str,
    pub debian_tab: &'static str,
    pub apt_add_repository: &'static str,
    pub apt_install: &'static str,
    pub trust_key: &'static str,
    pub add_repository: &'static str,
    pub install: &'static str,
    pub from_source: &'static str,
    pub questions_title: &'static str,
    pub questions: [Question; 4],
    pub license: &'static str,
    pub design_system: &'static str,
}

pub const EN: Text = Text {
    name: "Sklad",
    tagline: "Everything installed on Arch Linux, Debian and Ubuntu, in one list",
    language_name: "English",
    dark_mode: "Dark mode",
    headline: "Everything installed on Arch Linux, Debian and Ubuntu, in one list. Remove what you no longer use.",
    lead: "Package managers only tell you what they installed themselves. Sklad puts pacman, AUR and apt packages, Flatpak apps, Steam games, Rust toolchains, Cargo, npm and Python tools, Python environments, Docker containers and images, and loose files in one list. Sort it by last launch, read what each entry is, and uninstall it with a command you see before it runs.",
    install_action: "Install from the pacman or apt repository",
    source_action: "Source on GitHub",
    catalogue_shot: "en-catalogue.png",
    catalogue_alt: "The Sklad window: installed programs with icons, install date, size and last launch, and the card of the selected program",
    early_title: "Early version",
    early_text: "Tested on Arch Linux with KDE Plasma on Wayland, and against apt on Debian and Ubuntu. Read the command the window shows before you confirm it.",
    features_title: "What it does",
    features: [
        Feature {
            title: "One catalogue",
            text: "Programs, console tools, libraries, fonts, drivers and metapackages from every source, plus Docker containers, images and volumes, and Python environments.",
        },
        Feature {
            title: "Last launch",
            text: "From the KDE Plasma log and Steam, to find what you stopped using.",
        },
        Feature {
            title: "Orphans apart",
            text: "Unneeded dependencies and the packages you installed yourself, each in its own filter.",
        },
        Feature {
            title: "Safe removal",
            text: "pacman or apt checks the list first, and the exact command is always on screen.",
        },
        Feature {
            title: "Out of a metapackage",
            text: "The program goes, the rest of the metapackage stays.",
        },
        Feature {
            title: "Launch and copy",
            text: "Start a program from its card, and select and copy any text.",
        },
        Feature {
            title: "Four languages",
            text: "Ukrainian, English, French and Russian, light and dark themes.",
        },
    ],
    removal_title: "Safe removal",
    removal_shot: "en-removal.png",
    removal_alt: "The Sklad removal dialog for a program held by a metapackage: the package list and the command",
    removal_text: "The list of packages is computed on the dependency graph by the same rules as pacman -Rs or apt-get remove --auto-remove, then the package manager checks it. If the lists differ, nothing happens. Important system packages and files outside the home folder need a separate confirmation.",
    install_title: "Install",
    arch_tab: "Arch Linux",
    debian_tab: "Debian, Ubuntu",
    apt_add_repository: "Trust the key and add the apt repository; it needs Debian 12, Ubuntu 22.04 or newer, and running the commands again changes nothing:",
    apt_install: "Install it like any package; updates come with apt upgrade:",
    trust_key: "Trust the key that signs the packages, once:",
    add_repository: "Add the repository to /etc/pacman.conf; the command does nothing if it is already there:",
    install: "Install it like any package; updates come with pacman -Syu:",
    from_source: "Or build it from source:",
    questions_title: "Questions",
    questions: [
        Question {
            title: "How do I find unused packages on Arch Linux, Debian or Ubuntu?",
            answer: "pacman -Qdtq and apt autoremove find orphan dependencies, but not the programs you installed and stopped using. Sklad sorts everything by the last launch Plasma and Steam recorded, so programs you have not opened for months come first.",
        },
        Question {
            title: "How is it different from Pamac, Octopi, Synaptic or BleachBit?",
            answer: "Pamac, Octopi and Synaptic manage the packages of the distribution, and BleachBit cleans caches. Sklad lists what pacman, the AUR, apt, Flatpak, Steam, Docker and language package managers installed, in one list, and removes each entry through the manager that installed it.",
        },
        Question {
            title: "Does it need KDE Plasma?",
            answer: "It is tested on KDE Plasma on Wayland. The last launch of programs comes from the Plasma log, so on other desktops it is known only for Steam games; the catalogue and removal work anywhere.",
        },
        Question {
            title: "Is it safe?",
            answer: "Sklad removes nothing without showing the command, and pacman or apt checks the list before it runs. It is an early version, so read the command before you confirm.",
        },
    ],
    license: "MIT license",
    design_system: "Operator UI design system",
};

pub const UK: Text = Text {
    name: "Склад",
    tagline: "Усе встановлене в Arch Linux, Debian і Ubuntu одним списком",
    language_name: "Українська (Ukrainian)",
    dark_mode: "Темна тема",
    headline: "Усе встановлене в Arch Linux, Debian і Ubuntu одним списком. Видаляйте те, чим не користуєтеся.",
    lead: "Менеджери пакетів показують лише те, що встановили самі. Склад збирає в один список пакети pacman, AUR і apt, програми Flatpak, ігри Steam, інструментарії Rust, інструменти Cargo, npm і Python, середовища Python, контейнери й образи Docker та файли поза менеджерами. Відсортуйте його за останнім запуском, прочитайте, що це таке, і видаліть командою, яку видно ще до запуску.",
    install_action: "Встановити з репозиторію pacman або apt",
    source_action: "Код на GitHub",
    catalogue_shot: "uk-catalogue.png",
    catalogue_alt: "Вікно Складу: список встановлених програм з іконками, датою встановлення, розміром і останнім запуском та картка вибраної програми",
    early_title: "Рання версія",
    early_text: "Перевірено в Arch Linux із KDE Plasma на Wayland і звірено з apt у Debian та Ubuntu. Читайте команду у вікні, перш ніж підтвердити.",
    features_title: "Що він уміє",
    features: [
        Feature {
            title: "Один каталог",
            text: "Програми, консольні інструменти, бібліотеки, шрифти, драйвери й метапакети з усіх джерел, а ще контейнери, образи й томи Docker і середовища Python.",
        },
        Feature {
            title: "Останній запуск",
            text: "За журналом KDE Plasma і даними Steam, щоб знайти те, чим давно не користуєтеся.",
        },
        Feature {
            title: "Непотрібне окремо",
            text: "Непотрібні залежності й пакети, встановлені вами, мають власні фільтри.",
        },
        Feature {
            title: "Безпечне видалення",
            text: "Список спершу перевіряє pacman або apt, а точна команда завжди на екрані.",
        },
        Feature {
            title: "Видалення з метапакета",
            text: "Програма зникає, решта метапакета лишається.",
        },
        Feature {
            title: "Запуск і копіювання",
            text: "Запускайте програму з її картки, виділяйте й копіюйте будь-який текст.",
        },
        Feature {
            title: "Чотири мови",
            text: "Українська, англійська, французька й російська, світла й темна теми.",
        },
    ],
    removal_title: "Безпечне видалення",
    removal_shot: "uk-removal.png",
    removal_alt: "Вікно видалення Складу для програми з метапакета: список пакетів і команда",
    removal_text: "Список пакетів рахується на графі залежностей за тими ж правилами, що й у pacman -Rs або apt-get remove --auto-remove, а потім його перевіряє менеджер пакетів. Якщо списки різняться, нічого не відбувається. Важливі системні пакети й файли поза домашньою текою потребують окремого підтвердження.",
    install_title: "Встановлення",
    arch_tab: "Arch Linux",
    debian_tab: "Debian, Ubuntu",
    apt_add_repository: "Довірте ключ і додайте репозиторій apt; потрібні Debian 12, Ubuntu 22.04 або новіші, а повторний запуск команд нічого не змінить:",
    apt_install: "Встановіть як звичайний пакет; оновлення прийдуть разом із apt upgrade:",
    trust_key: "Один раз довірте ключ, яким підписано пакети:",
    add_repository: "Додайте репозиторій у /etc/pacman.conf; якщо він там уже є, команда нічого не змінить:",
    install: "Встановіть як звичайний пакет; оновлення прийдуть разом із pacman -Syu:",
    from_source: "Або зберіть із коду:",
    questions_title: "Питання",
    questions: [
        Question {
            title: "Як знайти непотрібні пакети в Arch Linux, Debian чи Ubuntu?",
            answer: "pacman -Qdtq і apt autoremove знайдуть осиротілі залежності, але не програми, які ви встановили й закинули. Склад сортує все за останнім запуском, який записали Plasma і Steam, тож програми, яких ви не відкривали місяцями, будуть угорі.",
        },
        Question {
            title: "Чим він відрізняється від Pamac, Octopi, Synaptic чи BleachBit?",
            answer: "Pamac, Octopi і Synaptic керують пакетами дистрибутива, BleachBit чистить кеші. Склад показує встановлене через pacman, AUR, apt, Flatpak, Steam, Docker і менеджери пакетів мов в одному списку та видаляє кожен запис тим менеджером, який його встановив.",
        },
        Question {
            title: "Чи потрібна KDE Plasma?",
            answer: "Програму перевірено в KDE Plasma на Wayland. Останній запуск програм береться з журналу Plasma, тож в інших середовищах він відомий лише для ігор Steam; каталог і видалення від середовища не залежать.",
        },
        Question {
            title: "Чи це безпечно?",
            answer: "Склад нічого не видаляє, не показавши команду, а pacman або apt перевіряє список ще до запуску. Це рання версія, тож читайте команду, перш ніж підтвердити.",
        },
    ],
    license: "Ліцензія MIT",
    design_system: "Дизайн-система Operator UI",
};

pub const FR: Text = Text {
    name: "Sklad",
    tagline: "Tout ce qui est installé sur Arch Linux, Debian et Ubuntu, dans une seule liste",
    language_name: "Français (French)",
    dark_mode: "Thème sombre",
    headline: "Tout ce qui est installé sur Arch Linux, Debian et Ubuntu, dans une seule liste. Supprimez ce dont vous ne vous servez plus.",
    lead: "Les gestionnaires de paquets ne montrent que ce qu'ils ont installé eux-mêmes. Sklad réunit dans une seule liste les paquets pacman, AUR et apt, les applications Flatpak, les jeux Steam, les chaînes d'outils Rust, les outils Cargo, npm et Python, les environnements Python, les conteneurs et images Docker, et les fichiers hors gestionnaires. Triez-la par dernier lancement, lisez ce qu'est chaque entrée, et désinstallez-la avec une commande que vous voyez avant qu'elle ne s'exécute.",
    install_action: "Installer depuis le dépôt pacman ou apt",
    source_action: "Code sur GitHub",
    catalogue_shot: "fr-catalogue.png",
    catalogue_alt: "La fenêtre de Sklad : les programmes installés avec leur icône, leur date d'installation, leur taille et leur dernier lancement, et la fiche du programme choisi",
    early_title: "Version précoce",
    early_text: "Testé sur Arch Linux avec KDE Plasma sous Wayland, et comparé à apt sur Debian et Ubuntu. Lisez la commande affichée par la fenêtre avant de la confirmer.",
    features_title: "Ce qu'il fait",
    features: [
        Feature {
            title: "Un seul catalogue",
            text: "Programmes, outils en console, bibliothèques, polices, pilotes et métapaquets de toutes les sources, ainsi que les conteneurs, images et volumes Docker et les environnements Python.",
        },
        Feature {
            title: "Dernier lancement",
            text: "D'après le journal de KDE Plasma et Steam, pour trouver ce que vous n'utilisez plus.",
        },
        Feature {
            title: "Les orphelins à part",
            text: "Les dépendances inutiles et les paquets que vous avez installés vous-même, chacun dans son filtre.",
        },
        Feature {
            title: "Suppression sûre",
            text: "pacman ou apt vérifie d'abord la liste, et la commande exacte est toujours à l'écran.",
        },
        Feature {
            title: "Hors d'un métapaquet",
            text: "Le programme part, le reste du métapaquet reste.",
        },
        Feature {
            title: "Lancer et copier",
            text: "Lancez un programme depuis sa fiche, sélectionnez et copiez n'importe quel texte.",
        },
        Feature {
            title: "Quatre langues",
            text: "Ukrainien, anglais, français et russe, thèmes clair et sombre.",
        },
    ],
    removal_title: "Suppression sûre",
    removal_shot: "fr-removal.png",
    removal_alt: "La fenêtre de suppression de Sklad pour un programme retenu par un métapaquet : la liste des paquets et la commande",
    removal_text: "La liste des paquets est calculée sur le graphe des dépendances selon les mêmes règles que pacman -Rs ou apt-get remove --auto-remove, puis le gestionnaire de paquets la vérifie. Si les listes diffèrent, rien ne se passe. Les paquets système importants et les fichiers hors du dossier personnel demandent une confirmation à part.",
    install_title: "Installation",
    arch_tab: "Arch Linux",
    debian_tab: "Debian, Ubuntu",
    apt_add_repository: "Faites confiance à la clé et ajoutez le dépôt apt ; il faut Debian 12, Ubuntu 22.04 ou plus récent, et relancer les commandes ne change rien :",
    apt_install: "Installez-le comme n'importe quel paquet ; les mises à jour arrivent avec apt upgrade :",
    trust_key: "Faites confiance une fois à la clé qui signe les paquets :",
    add_repository: "Ajoutez le dépôt à /etc/pacman.conf ; la commande ne fait rien s'il y est déjà :",
    install: "Installez-le comme n'importe quel paquet ; les mises à jour arrivent avec pacman -Syu :",
    from_source: "Ou compilez-le depuis le code source :",
    questions_title: "Questions",
    questions: [
        Question {
            title: "Comment trouver les paquets inutilisés sur Arch Linux, Debian ou Ubuntu ?",
            answer: "pacman -Qdtq et apt autoremove trouvent les dépendances orphelines, mais pas les programmes que vous avez installés puis délaissés. Sklad trie tout selon le dernier lancement enregistré par Plasma et Steam, si bien que les programmes que vous n'avez pas ouverts depuis des mois arrivent en tête.",
        },
        Question {
            title: "En quoi est-il différent de Pamac, Octopi, Synaptic ou BleachBit ?",
            answer: "Pamac, Octopi et Synaptic gèrent les paquets de la distribution, et BleachBit nettoie les caches. Sklad montre dans une seule liste ce qu'ont installé pacman, l'AUR, apt, Flatpak, Steam, Docker et les gestionnaires de paquets des langages, et supprime chaque entrée par le gestionnaire qui l'a installée.",
        },
        Question {
            title: "Faut-il KDE Plasma ?",
            answer: "Il est testé sur KDE Plasma sous Wayland. Le dernier lancement des programmes vient du journal de Plasma, donc sur les autres bureaux il n'est connu que pour les jeux Steam ; le catalogue et la suppression fonctionnent partout.",
        },
        Question {
            title: "Est-ce sûr ?",
            answer: "Sklad ne supprime rien sans montrer la commande, et pacman ou apt vérifie la liste avant l'exécution. C'est une version précoce : lisez la commande avant de confirmer.",
        },
    ],
    license: "Licence MIT",
    design_system: "Système de design Operator UI",
};

pub const RU: Text = Text {
    name: "Склад",
    tagline: "Всё установленное в Arch Linux, Debian и Ubuntu одним списком",
    language_name: "Русский (Russian)",
    dark_mode: "Тёмная тема",
    headline: "Всё установленное в Arch Linux, Debian и Ubuntu одним списком. Удаляйте то, чем не пользуетесь.",
    lead: "Менеджеры пакетов показывают только то, что установили сами. Склад собирает в один список пакеты pacman, AUR и apt, программы Flatpak, игры Steam, инструментарии Rust, инструменты Cargo, npm и Python, окружения Python, контейнеры и образы Docker и файлы вне менеджеров. Отсортируйте его по последнему запуску, прочитайте, что это такое, и удалите командой, которую видно ещё до запуска.",
    install_action: "Установить из репозитория pacman или apt",
    source_action: "Код на GitHub",
    catalogue_shot: "ru-catalogue.png",
    catalogue_alt: "Окно Склада: список установленных программ с иконками, датой установки, размером и последним запуском и карточка выбранной программы",
    early_title: "Ранняя версия",
    early_text: "Проверено в Arch Linux с KDE Plasma на Wayland и сверено с apt в Debian и Ubuntu. Читайте команду в окне, прежде чем подтвердить.",
    features_title: "Что он умеет",
    features: [
        Feature {
            title: "Один каталог",
            text: "Программы, консольные инструменты, библиотеки, шрифты, драйверы и метапакеты из всех источников, а также контейнеры, образы и тома Docker и окружения Python.",
        },
        Feature {
            title: "Последний запуск",
            text: "По журналу KDE Plasma и данным Steam, чтобы найти то, чем давно не пользуетесь.",
        },
        Feature {
            title: "Ненужное отдельно",
            text: "Ненужные зависимости и пакеты, установленные вами, в собственных фильтрах.",
        },
        Feature {
            title: "Безопасное удаление",
            text: "Список сначала проверяет pacman или apt, а точная команда всегда на экране.",
        },
        Feature {
            title: "Удаление из метапакета",
            text: "Программа уходит, остальной метапакет остаётся.",
        },
        Feature {
            title: "Запуск и копирование",
            text: "Запускайте программу из её карточки, выделяйте и копируйте любой текст.",
        },
        Feature {
            title: "Четыре языка",
            text: "Украинский, английский, французский и русский, светлая и тёмная темы.",
        },
    ],
    removal_title: "Безопасное удаление",
    removal_shot: "ru-removal.png",
    removal_alt: "Окно удаления Склада для программы из метапакета: список пакетов и команда",
    removal_text: "Список пакетов считается на графе зависимостей по тем же правилам, что и в pacman -Rs или apt-get remove --auto-remove, а затем его проверяет менеджер пакетов. Если списки различаются, ничего не происходит. Важные системные пакеты и файлы вне домашней папки требуют отдельного подтверждения.",
    install_title: "Установка",
    arch_tab: "Arch Linux",
    debian_tab: "Debian, Ubuntu",
    apt_add_repository: "Доверьте ключ и добавьте репозиторий apt; нужны Debian 12, Ubuntu 22.04 или новее, а повторный запуск команд ничего не изменит:",
    apt_install: "Установите как обычный пакет; обновления придут вместе с apt upgrade:",
    trust_key: "Один раз доверьте ключ, которым подписаны пакеты:",
    add_repository: "Добавьте репозиторий в /etc/pacman.conf; если он там уже есть, команда ничего не изменит:",
    install: "Установите как обычный пакет; обновления придут вместе с pacman -Syu:",
    from_source: "Или соберите из исходного кода:",
    questions_title: "Вопросы",
    questions: [
        Question {
            title: "Как найти ненужные пакеты в Arch Linux, Debian или Ubuntu?",
            answer: "pacman -Qdtq и apt autoremove найдут осиротевшие зависимости, но не программы, которые вы установили и забросили. Склад сортирует всё по последнему запуску, который записали Plasma и Steam, так что программы, которые вы не открывали месяцами, окажутся вверху.",
        },
        Question {
            title: "Чем он отличается от Pamac, Octopi, Synaptic или BleachBit?",
            answer: "Pamac, Octopi и Synaptic управляют пакетами дистрибутива, BleachBit чистит кэши. Склад показывает установленное через pacman, AUR, apt, Flatpak, Steam, Docker и менеджеры пакетов языков в одном списке и удаляет каждую запись тем менеджером, который её установил.",
        },
        Question {
            title: "Нужна ли KDE Plasma?",
            answer: "Программа проверена в KDE Plasma на Wayland. Последний запуск программ берётся из журнала Plasma, поэтому в других окружениях он известен только для игр Steam; каталог и удаление от окружения не зависят.",
        },
        Question {
            title: "Это безопасно?",
            answer: "Склад ничего не удаляет, не показав команду, а pacman или apt проверяет список ещё до запуска. Это ранняя версия, поэтому читайте команду, прежде чем подтвердить.",
        },
    ],
    license: "Лицензия MIT",
    design_system: "Дизайн-система Operator UI",
};
