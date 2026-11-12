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
- `cargo test -- --ignored --test-threads=1` compares the plans with `pacman` on your machine. It only reads and
  takes about 4 minutes.
- A fix comes with a test that fails without it.
- No test may remove anything real: window tests use a fake backend.

## Interface text

Every word of the interface lives in `src/i18n/`. Add it to both `uk.rs` and `en.rs`; a string missing from a
language does not build, and `tests/one_catalogue.rs` refuses interface words anywhere else.

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
It builds the package in an Arch Linux container, signs it with the key in the `PACKAGER_GPG_KEY` secret, publishes a GitHub release, and refreshes the website on GitHub Pages together with the pacman repository under `/x86_64/`.
The public half of the key is `packaging/sklad.asc`.
