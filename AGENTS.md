# Agent Instructions

Rules for AI agents contributing to this repository.
The interface follows the [Operator UI design system](https://github.com/pavver/operator-design-system); most of these rules come from that project.

## Scope

* sklad implements the design system in egui in [src/ui/operator](src/ui/operator); the rest of [src/ui](src/ui) only composes those components.
* Do not invent a component, a variant, or a token the design system does not have.
  Report the gap instead, and compose from existing components only when the maintainer agrees.
* Token values come from the `operator-ui-tokens` crate, pinned to a commit in [Cargo.toml](Cargo.toml).
  Moving to a newer commit of the design system is a change of its own.

## Draft components

The design system marks these components as Draft, and sklad uses them anyway:

* `DataTable` ([src/ui/operator/data_table.rs](src/ui/operator/data_table.rs)): the catalogue table.
  It departs from the specification, with the maintainer's agreement: the first cell may lead with a square slot, which holds the `Avatar` of the program.
  That `Avatar` departs from its own specification too, with the same agreement: a Docker row, which has no image, shows the icon of its kind in place of the initial.
* `PropertyList` (in [src/ui/operator/content.rs](src/ui/operator/content.rs)): the facts on the card.
* `Toolbar` (in [src/ui/operator/layout.rs](src/ui/operator/layout.rs)): the bar above the table; its overflow menu is not ported.

Reviewed components that sklad uses with a departure the maintainer agreed to:

* `CheckboxGroup` ([src/ui/operator/checkbox_group.rs](src/ui/operator/checkbox_group.rs)): the filters of the side bar.
  It is used synchronously, and each tile carries a neutral `Counter` before its indicator box, which the specification does not have yet.
  Section tiles show the icon of the system theme and source tiles the logo of the source, not a Material Symbols glyph; see [Icons](#icons).

Their specifications may still change, so expect to rework them when the design system marks them Reviewed.

## Language and Formatting

* All files are written in English.
  The exceptions are the interface text in [src/i18n](src/i18n) in each of its languages, the test strings that check it, [README.uk.md](README.uk.md), [README.fr.md](README.fr.md), [README.ru.md](README.ru.md), and the translated website in [site/src/text.rs](site/src/text.rs), [site/uk.html](site/uk.html), [site/fr.html](site/fr.html) and [site/ru.html](site/ru.html).
* Every word of the interface lives in [src/i18n](src/i18n), in every language: Ukrainian, English, French and Russian.
* The website in [site](site) is composed only from the `operator-ui` web components of the design system, pinned to the same commit; its CSS only lays the page out.
  [scripts/build_site.sh](scripts/build_site.sh) builds it with Trunk.
* In Markdown files, every sentence is on its own line.
* Files are referenced with relative paths.

## Units

* Spatial values use Design Units (`1 DU = 4 logical points`) through `du()` and the `dim` tokens.
* Font sizes and line heights use Type Units (`1 TU = 1 logical point`) through the `Type` scale.
* Raw point values are prohibited in component code.
  The only exceptions are the unit definitions themselves and geometry measured at runtime (pointer positions, element sizes).

## Geometry

* Corner radius is `0 DU` unless the component specification lists a round exception.
* Standard border `0.25 DU`, emphasis border `0.5 DU`, always drawn inside the bounds.

## Icons

* Use Material Symbols Sharp.
  Icons of the design system come from the tokens crate; any other one is imported into [assets/icons](assets/icons) through [assets/icons/icons.toml](assets/icons/icons.toml) and [scripts/import_icons.py](scripts/import_icons.py).
* `python3 scripts/import_icons.py --check` must pass.
* Deliberate departures, so the person finds a filter by a picture they already know:
  * Section tiles of the side bar show the matching icon of the system icon theme (for example `applications-graphics`), in its own colours, as in the menu of the desktop.
    The Material Symbols glyph stays as the fallback when the theme has no such icon.
  * Source tiles show the logo of the source (Arch Linux, Debian, Flatpak, Steam, Rust, Node.js, Python, Docker), built into sklad so it is the same on every system and never taken from the theme.
    The logos come from [Simple Icons](https://simpleicons.org) through the same manifest and script, and are drawn in one colour like any icon; a source without a logo keeps a Material Symbols glyph.
  * Origin tiles always use Material Symbols.
  * Do not extend this to other components without the maintainer's agreement.

## Behavior

* Follow the [states specification](https://github.com/pavver/operator-design-system/blob/65712c51230acba7399b8ae412f6c9658932999f/docs/foundations/states.md) of the design system for every interactive component.
* Pending blocks input, results do not, errors roll back with the same animation as the forward change.

## Rust

* No `.unwrap()` or `.expect()`.
* Run `cargo fmt`, `cargo clippy --all-targets` and `cargo test` before committing; the code must be warning-free.

## Git

* Conventional Commits in English.
* No automated or AI signatures in commit messages.
