//! Every word of the website, in English and Ukrainian.

/// The commands are the same in both languages.
pub const TRUST_KEY: &str = "curl -sL https://pavver.github.io/sklad/sklad.asc | sudo pacman-key --add -
sudo pacman-key --lsign-key EB0FE0C7F0FEBBD1A40F8F93A7A3ED14D1C98D8D";
/// Appends the repository to pacman.conf unless it is already there.
pub const ADD_REPOSITORY: &str = r"grep -q '^\[sklad\]' /etc/pacman.conf ||
  printf '\n[sklad]\nServer = https://pavver.github.io/sklad/$arch\n' | sudo tee -a /etc/pacman.conf >/dev/null";
pub const INSTALL: &str = "sudo pacman -Sy sklad";
pub const FROM_SOURCE: &str = "cargo install --git https://github.com/pavver/sklad";

/// Each page in its language: the name in that language, then in English, so a search finds it
/// either way. English needs no second name.
pub const PAGES: [(&str, &str); 2] = [("Українська (Ukrainian)", "uk.html"), ("English", "./")];

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
    tagline: "Everything installed on Arch Linux, in one list",
    language_name: "English",
    dark_mode: "Dark mode",
    headline: "Everything installed on Arch Linux, in one list. Remove what you no longer use.",
    lead: "Package managers only tell you what they installed themselves. Sklad puts pacman and AUR packages, Flatpak apps, Steam games, Rust toolchains, Cargo, npm and Python tools, and loose files in one list. Sort it by last launch, read what each entry is, and uninstall it with a command you see before it runs.",
    install_action: "Install from the pacman repository",
    source_action: "Source on GitHub",
    catalogue_shot: "en-catalogue.png",
    catalogue_alt: "The Sklad window: installed programs with icons, install date, size and last launch, and the card of the selected program",
    early_title: "Early version",
    early_text: "Tested on Arch Linux with KDE Plasma on Wayland. Read the command the window shows before you confirm it.",
    features_title: "What it does",
    features: [
        Feature {
            title: "One catalogue",
            text: "Programs, console tools, libraries, fonts, drivers and metapackages from every source.",
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
            text: "pacman -Rp checks the list first, and the exact command is always on screen.",
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
            title: "Two languages",
            text: "Ukrainian and English, light and dark themes.",
        },
    ],
    removal_title: "Safe removal",
    removal_shot: "en-removal.png",
    removal_alt: "The Sklad removal dialog for a program held by a metapackage: the package list and the command",
    removal_text: "The list of packages is computed on the dependency graph by the same rules as pacman -Rs, then pacman -Rp checks it. If the lists differ, nothing happens. Important system packages and files outside the home folder need a separate confirmation.",
    install_title: "Install",
    trust_key: "Trust the key that signs the packages, once:",
    add_repository: "Add the repository to /etc/pacman.conf; the command does nothing if it is already there:",
    install: "Install it like any package; updates come with pacman -Syu:",
    from_source: "Or build it from source:",
    questions_title: "Questions",
    questions: [
        Question {
            title: "How do I find unused packages on Arch Linux?",
            answer: "pacman -Qdtq lists orphan dependencies, but not the programs you installed and stopped using. Sklad sorts everything by the last launch Plasma and Steam recorded, so programs you have not opened for months come first.",
        },
        Question {
            title: "How is it different from Pamac, Octopi or BleachBit?",
            answer: "Pamac and Octopi manage pacman packages, and BleachBit cleans caches. Sklad lists what pacman, the AUR, Flatpak, Steam and language package managers installed, in one list, and removes each entry through the manager that installed it.",
        },
        Question {
            title: "Does it need KDE Plasma?",
            answer: "It is tested on KDE Plasma on Wayland. The last launch of programs comes from the Plasma log, so on other desktops it is known only for Steam games; the catalogue and removal work anywhere.",
        },
        Question {
            title: "Is it safe?",
            answer: "Sklad removes nothing without showing the command, and pacman checks the list before it runs. It is an early version, so read the command before you confirm.",
        },
    ],
    license: "MIT license",
    design_system: "Operator UI design system",
};

pub const UK: Text = Text {
    name: "Склад",
    tagline: "Усе встановлене в Arch Linux одним списком",
    language_name: "Українська (Ukrainian)",
    dark_mode: "Темна тема",
    headline: "Усе встановлене в Arch Linux одним списком. Видаляйте те, чим не користуєтеся.",
    lead: "Менеджери пакетів показують лише те, що встановили самі. Склад збирає в один список пакети pacman і AUR, програми Flatpak, ігри Steam, інструментарії Rust, інструменти Cargo, npm і Python та файли поза менеджерами. Відсортуйте його за останнім запуском, прочитайте, що це таке, і видаліть командою, яку видно ще до запуску.",
    install_action: "Встановити з репозиторію pacman",
    source_action: "Код на GitHub",
    catalogue_shot: "uk-catalogue.png",
    catalogue_alt: "Вікно Складу: список встановлених програм з іконками, датою встановлення, розміром і останнім запуском та картка вибраної програми",
    early_title: "Рання версія",
    early_text: "Перевірено в Arch Linux із KDE Plasma на Wayland. Читайте команду у вікні, перш ніж підтвердити.",
    features_title: "Що він уміє",
    features: [
        Feature {
            title: "Один каталог",
            text: "Програми, консольні інструменти, бібліотеки, шрифти, драйвери й метапакети з усіх джерел.",
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
            text: "Список спершу перевіряє pacman -Rp, а точна команда завжди на екрані.",
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
            title: "Дві мови",
            text: "Українська та англійська, світла й темна теми.",
        },
    ],
    removal_title: "Безпечне видалення",
    removal_shot: "uk-removal.png",
    removal_alt: "Вікно видалення Складу для програми з метапакета: список пакетів і команда",
    removal_text: "Список пакетів рахується на графі залежностей за тими ж правилами, що й у pacman -Rs, а потім його перевіряє pacman -Rp. Якщо списки різняться, нічого не відбувається. Важливі системні пакети й файли поза домашньою текою потребують окремого підтвердження.",
    install_title: "Встановлення",
    trust_key: "Один раз довірте ключ, яким підписано пакети:",
    add_repository: "Додайте репозиторій у /etc/pacman.conf; якщо він там уже є, команда нічого не змінить:",
    install: "Встановіть як звичайний пакет; оновлення прийдуть разом із pacman -Syu:",
    from_source: "Або зберіть із коду:",
    questions_title: "Питання",
    questions: [
        Question {
            title: "Як знайти непотрібні пакети в Arch Linux?",
            answer: "pacman -Qdtq покаже осиротілі залежності, але не програми, які ви встановили й закинули. Склад сортує все за останнім запуском, який записали Plasma і Steam, тож програми, яких ви не відкривали місяцями, будуть угорі.",
        },
        Question {
            title: "Чим він відрізняється від Pamac, Octopi чи BleachBit?",
            answer: "Pamac і Octopi керують пакетами pacman, BleachBit чистить кеші. Склад показує встановлене через pacman, AUR, Flatpak, Steam і менеджери пакетів мов в одному списку та видаляє кожен запис тим менеджером, який його встановив.",
        },
        Question {
            title: "Чи потрібна KDE Plasma?",
            answer: "Програму перевірено в KDE Plasma на Wayland. Останній запуск програм береться з журналу Plasma, тож в інших середовищах він відомий лише для ігор Steam; каталог і видалення від середовища не залежать.",
        },
        Question {
            title: "Чи це безпечно?",
            answer: "Склад нічого не видаляє, не показавши команду, а pacman перевіряє список ще до запуску. Це рання версія, тож читайте команду, перш ніж підтвердити.",
        },
    ],
    license: "Ліцензія MIT",
    design_system: "Дизайн-система Operator UI",
};
