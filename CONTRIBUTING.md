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
