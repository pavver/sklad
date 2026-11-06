# Sklad

[Українська](README.uk.md)

A catalogue of everything installed on Arch Linux, from any source: repositories, AUR, Flatpak, Steam,
`cargo install`, npm, pnpm, pip, pipx, uv, rustup, files in `~/.local/bin`, `/usr/local/bin`, `/opt`
and your own launchers.

Each entry shows an icon, a description, the license, the website, the developer, the size, the install
date and the last launch. Entries can be sorted and removed.

## Running

```sh
cargo run --release
```

To add it to the application menu:

```sh
cargo install --path .
install -Dm644 packaging/sklad.desktop ~/.local/share/applications/sklad.desktop
```

## How removal works

1. The list of packages is computed on the dependency graph by the same rules as `pacman -Rs`.
2. A metapackage is removed together with the program. The rest of its packages are marked as explicitly
   installed and stay in the system.
3. `pacman -Rp` checks the list before anything happens. If the lists differ, the action stops.
4. `pacman` receives the packages by name, so only what the window shows is removed.
5. Important system packages and files outside the home folder need a separate confirmation.

Files from the home folder go to the bin. Files from `/opt` and `/usr/local/bin` are deleted for good.
Language packages are removed by the manager that installed them.

## Last launch

Launch times come from the Plasma log and from Steam manifests. Plasma only records launches from the
menu, the panel and KRunner, so "no record" does not mean a program is unused. Launches of console programs
are not tracked.

## Language

The interface is in Ukrainian and English. The language comes from the system; the `EN` / `УК` button in the
top bar switches it and remembers the choice in `~/.config/sklad/language`. Program descriptions are read in
the chosen language too, so the catalogue is read again after a switch. All strings live in `src/i18n/`.

## Checks

```sh
cargo test
cargo clippy --all-targets
cargo test -- --ignored --test-threads=1   # only reads the live system, about 4 minutes
```

Window tests use a fake backend, so they cannot remove a real package. Screenshots are saved to
`target/shots`.

The interface follows the Operator UI design system. Fonts: Cascadia Mono (SIL OFL 1.1) and Phosphor (MIT),
their licenses are in `assets/`.

## License

MIT, see [LICENSE](LICENSE). Changes come through pull requests.
