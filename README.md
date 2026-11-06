<div align="center">

<img src="assets/sklad.svg" width="96" alt="">

# Sklad

**See everything installed on your Arch Linux system, from any source, and remove what you no longer need.**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)
![Arch Linux](https://img.shields.io/badge/Arch_Linux-1793D1?logo=archlinux&logoColor=white)
![egui](https://img.shields.io/badge/UI-egui-5a67d8)

[Українська](README.uk.md)

![The catalogue with the card of a program](docs/screenshots/en-catalogue.png)

</div>

Package managers tell you what they installed. Sklad puts it all in one list: pacman, AUR, Flatpak, Steam,
language tools and loose files. Sort by last launch to find what has gone unused, read what it is, and remove
it with a command you can see first.

> [!WARNING]
> Early version. Tests use a fake backend and never run a real removal, so read the command the window shows
> before you confirm it. Tested on Arch Linux with KDE Plasma on Wayland.

## Features

- **One catalogue** of programs, console tools, libraries, fonts, drivers and metapackages.
- **Details for each entry:** icon, description, license, website, developer, size, install date.
- **Last launch** from the Plasma log and Steam, to surface what you stopped using.
- **Filters:** by section, by source, explicit or dependency, orphans, and search.
- **Removal out of a metapackage:** the program goes, the rest of the metapackage stays.
- **Nothing hidden:** the exact command is on screen and can be copied instead of run.
- **Ukrainian and English**, light and dark themes.

## What it finds

| Source | Where it looks | How an entry is removed |
| --- | --- | --- |
| Repositories, AUR | the pacman database | `pacman -Rn` with a checked list |
| Flatpak | system and user installations | `flatpak uninstall` |
| Steam | library manifests | the Steam client opens and asks |
| Cargo | `cargo install` records | `cargo uninstall` |
| Node.js | npm and pnpm global packages | `npm uninstall -g`, `pnpm remove -g` |
| Python | `pip --user`, pipx, uv tools, uv interpreters | the manager that installed it |
| Rustup | toolchains | `rustup toolchain uninstall` |
| Manual | `~/.local/bin`, `/usr/local/bin`, `/opt`, your launchers | home files to the bin, system files for good |

Python records in the system folder that no package owns are shown, but removing them from here is off.

## Safe removal

![The removal dialog for a program held by a metapackage](docs/screenshots/en-removal.png)

1. The list of packages is computed on the dependency graph by the same rules as `pacman -Rs`.
2. A metapackage is removed together with the program. The rest of its packages are marked as explicitly
   installed and stay in the system.
3. `pacman -Rp` checks the list before anything happens. If the lists differ, the action stops.
4. `pacman` receives the packages by name, so only what the window shows is removed.
5. Important system packages and files outside the home folder need a separate confirmation.

## Language tools

![Rust toolchains in the light theme](docs/screenshots/en-tools.png)

Toolchains and global packages take gigabytes without appearing in any package list. Sklad shows them next to
everything else and warns before removing something others rely on, such as the default Rust toolchain.

## Install

You need Rust, `pacman`, and `polkit` for removals that ask for a password.

```sh
git clone https://github.com/pavver/sklad
cd sklad
cargo install --path .
install -Dm644 packaging/sklad.desktop ~/.local/share/applications/sklad.desktop
install -Dm644 assets/sklad.svg ~/.local/share/icons/hicolor/scalable/apps/sklad.svg
```

Or run it without installing:

```sh
cargo run --release
```

## Good to know

- Plasma records launches from the menu, the panel and KRunner only, so "no record" does not mean a program is
  unused. Launches of console programs are not tracked.
- Sizes are the size of the files. On a compressed file system such as ZFS, less space is freed.
- The language comes from the system. The `EN` / `УК` button switches it and remembers the choice in
  `~/.config/sklad/language`. Descriptions follow the language, so the catalogue is read again.

## Contributing

Changes come through pull requests, see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE). Fonts: Cascadia Mono (SIL OFL 1.1) and Phosphor (MIT), their licenses are in `assets/`.
