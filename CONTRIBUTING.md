# Contributing

Changes come through pull requests: fork the repository, work on a branch, open a pull request against `main`.
Issues and pull requests are welcome in English or Ukrainian.

## Before opening a pull request

```sh
cargo fmt --check
cargo clippy --all-targets
cargo test
```

- Window tests need a GPU or a software Vulkan driver.
- `cargo test -- --ignored --test-threads=1` compares the plans with `pacman` or `apt` on your machine.
  It only reads and takes a few minutes.
- A fix comes with a test that fails without it.
- No test may remove anything real: window tests use a fake backend.

## Interface text

Every word of the interface lives in `src/i18n/`.
Add it to `uk.rs`, `en.rs`, `fr.rs` and `ru.rs`; a string missing from a language does not build, and `tests/one_catalogue.rs` refuses interface words anywhere else.

## Screenshots

The pictures in `docs/screenshots` come from an invented catalogue, so they show no real machine:

```sh
cargo test --test readme_pictures -- --ignored
```

## Releases

A release is a tag `vX.Y.Z` on `main`.
Before tagging, the same version goes into `Cargo.toml`, `packaging/arch/PKGBUILD` and a new `<release>` of `packaging/io.github.pavver.sklad.metainfo.xml`; `tests/repository.rs` refuses versions that differ.

```sh
git tag v0.2.0
git push origin v0.2.0
```

The tag starts `.github/workflows/release.yml`.
It builds the pacman package in an Arch Linux container and the Debian package in a Debian 12 container with `scripts/build_deb.sh`.
Both are signed with the key in the `PACKAGER_GPG_KEY` secret and published as a GitHub release.
The website on GitHub Pages is refreshed together with the pacman repository under `/x86_64/` and the apt repository under `/debian/`, made by `scripts/apt_repository.sh`.
The public half of the key is `packaging/sklad.asc`.
