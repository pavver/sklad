<div align="center">

<img src="assets/sklad.svg" width="96" alt="Sklad logo">

# Sklad

**A desktop app for Arch Linux that shows everything installed, from any source, and removes what you no longer use.**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![CI](https://github.com/pavver/sklad/actions/workflows/ci.yml/badge.svg)](https://github.com/pavver/sklad/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)
![Arch Linux](https://img.shields.io/badge/Arch_Linux-1793D1?logo=archlinux&logoColor=white)
![egui](https://img.shields.io/badge/UI-egui-5a67d8)

[Website](https://pavver.github.io/sklad/) · [Українська](README.uk.md)

![The Sklad catalogue: installed programs with icons, sizes and last launch, and the card of a program](docs/screenshots/en-catalogue.png)

</div>

Package managers only tell you what they installed themselves.
Sklad puts pacman and AUR packages, Flatpak apps, Steam games, Rust toolchains, Cargo, npm and Python tools, and loose files in one list.
Sort it by last launch to find what has gone unused, read what each entry is, and uninstall it with a command you see before it runs.
It is a graphical way to find unused packages, orphan dependencies and forgotten developer tools, and to free disk space on Arch Linux.

> [!WARNING]
> Early version.
> Tests use a fake backend and never run a real removal, so read the command the window shows before you confirm it.
> Tested on Arch Linux with KDE Plasma on Wayland.

## Features

- **One catalogue** of programs, console tools, libraries, fonts, drivers and metapackages.
- **Details for each entry:** icon, description, license, website, developer, size, install date.
- **Last launch** from the Plasma log and Steam, to surface what you stopped using.
- **Filters:** by section, by source, packages you installed yourself, dependencies, unneeded dependencies (orphans), and search.
- **Removal out of a metapackage:** the program goes, the rest of the metapackage stays.
- **Nothing hidden:** the exact command is on screen and can be copied instead of run.
- **Launch** a program from its card, and select and copy any text in the window.
- **Ukrainian and English**, light and dark themes.

## What it finds

| Source | Where it looks | How an entry is removed |
| --- | --- | --- |
| Repositories, AUR | the pacman database | `pacman -Rn` with a checked list |
| Flatpak | system and user installations | `flatpak uninstall`, with a password for system ones |
| Steam | library manifests | the Steam client opens and asks |
| Cargo | `cargo install` records | `cargo uninstall` |
| Node.js | npm and pnpm global packages | `npm uninstall -g`, `pnpm remove -g` |
| Python | `pip --user`, pipx, uv tools, uv interpreters | the manager that installed it |
| Rustup | toolchains | `rustup toolchain uninstall` |
| Outside managers | `~/.local/bin`, `/usr/local/bin`, `/opt`, your launchers | home files to the bin, system files for good |

Python records in the system folder that no package owns are shown, but removing them from here is off.

## Safe removal

![The removal dialog for a program held by a metapackage](docs/screenshots/en-removal.png)

1. The list of packages is computed on the dependency graph by the same rules as `pacman -Rs`.
2. A metapackage is removed together with the program.
   The rest of its packages are marked as installed by you and stay in the system.
3. `pacman -Rp` checks the list before anything happens.
   If the lists differ, the action stops.
4. `pacman` receives the packages by name, so only what the window shows is removed.
5. Important system packages and files outside the home folder need a separate confirmation.

## Language tools

![Rust toolchains in the light theme](docs/screenshots/en-tools.png)

Toolchains and global packages take gigabytes without appearing in any package list.
Sklad shows them next to everything else and warns before removing something others rely on, such as the default Rust toolchain.

## Install

Sklad has its own pacman repository, with packages signed by the key `EB0FE0C7F0FEBBD1A40F8F93A7A3ED14D1C98D8D`.
Trust the key once:

```sh
curl -sL https://pavver.github.io/sklad/sklad.asc | sudo pacman-key --add -
sudo pacman-key --lsign-key EB0FE0C7F0FEBBD1A40F8F93A7A3ED14D1C98D8D
```

Add the repository to `/etc/pacman.conf`; the command does nothing if it is already there:

```sh
grep -q '^\[sklad\]' /etc/pacman.conf ||
  printf '\n[sklad]\nServer = https://pavver.github.io/sklad/$arch\n' | sudo tee -a /etc/pacman.conf >/dev/null
```

Then install it; updates come with `pacman -Syu`:

```sh
sudo pacman -Sy sklad
```

### From source

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

## Compared with other tools

- `pacman -Qdtq` lists orphan dependencies, but not the programs you installed and stopped using.
- Pamac and Octopi manage pacman packages; Sklad also lists Flatpak, Steam and language package managers, and sorts everything by last launch.
- BleachBit cleans caches and temporary files; Sklad removes whole programs through the manager that installed them.

## Good to know

- Plasma records launches from the menu, the panel and KRunner only, so "no record" does not mean a program is unused.
  Launches of console programs are not tracked.
- Sizes are the size of the files.
  On a compressed file system such as ZFS, less space is freed.
- The language comes from the system.
  The language choice in the header switches it and remembers it in `~/.config/sklad/language`.
  A language is found by its own name or by its English one.
  Descriptions follow the language, so the catalogue is read again.

## Contributing

Changes come through pull requests, see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE).
The interface follows the [Operator UI](https://github.com/pavver/operator-design-system) design system.
Third-party assets (the Cascadia Code font, Material Symbols icons, the day/night toggle) keep their own licenses, listed in [NOTICE.md](NOTICE.md).
