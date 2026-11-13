//! The card of one item: what Discover shows about a program, plus what cleaning needs.

use eframe::egui::{self, Frame, Id, Margin, ScrollArea, Sense, Ui, vec2};

use super::format;
use super::operator::button::{ActionButton, IconButton};
use super::operator::code::CodeBlock;
use super::operator::content::{ChipColor, Property, avatar, chip, chip_set, property_list};
use super::operator::icon;
use super::operator::layout::section_title;
use super::operator::state::Look;
use super::operator::tokens::{Palette, Type, dim, du, text};
use super::pictures::Pictures;
use crate::i18n::Text;
use crate::model::{Access, DockerObject, Item, Reason, Removal, Usage};

pub enum Action {
    Launch,
    Remove,
    Open(String),
    /// Go to another package by name.
    Follow(String),
    /// Close the card, leaving nothing selected.
    Close,
}

/// How many neighbours are listed before the list is cut.
const LINK_LIMIT: usize = 60;

fn removal_note(removal: &Removal, text: &Text) -> String {
    let note = match removal {
        Removal::Package(system) => return (text.by_package)(system.manager()),
        Removal::Flatpak { .. } => text.by_flatpak,
        Removal::Steam { .. } => text.by_steam,
        Removal::Cargo { .. } => text.by_cargo,
        Removal::Docker { .. } => text.by_docker,
        Removal::Manager { .. } => text.by_language_manager,
        Removal::Unavailable { .. } => text.not_removable_here,
        Removal::Files {
            access: Access::User, ..
        } => text.files_to_bin,
        Removal::Files {
            access: Access::Root, ..
        } => text.files_for_good,
    };
    note.to_owned()
}

fn wrapped(ui: &mut Ui, content: impl Into<String>, kind: Type, colour: egui::Color32) {
    ui.add(egui::Label::new(text(content, kind, colour)).wrap());
}

fn links(ui: &mut Ui, names: &[String], action: &mut Option<Action>, text: &Text, palette: &Palette) {
    chip_set(ui, |ui| {
        for name in names.iter().take(LINK_LIMIT) {
            if chip(ui, name, ChipColor::Neutral, true, palette).clicked() {
                *action = Some(Action::Follow(name.clone()));
            }
        }
        if names.len() > LINK_LIMIT {
            let rest = (text.and_more)(names.len() - LINK_LIMIT);
            ui.label(super::operator::tokens::text(
                rest,
                Type::BodySmall,
                palette.text_secondary,
            ));
        }
    });
}

pub struct Card<'a> {
    pub item: &'a Item,
    pub command: Option<&'a str>,
    pub now: i64,
    /// How the last launch of this item is going.
    pub launch: Look,
    pub text: &'static Text,
}

/// The avatar, the title and the badges, with the button that closes the card. True when it was
/// pressed.
fn title_block(ui: &mut Ui, card: &Card<'_>, pictures: &mut Pictures, palette: &Palette) -> bool {
    let item = card.item;
    let text = card.text;
    let mut closed = false;
    ui.horizontal_top(|ui| {
        let side = du(16.0);
        let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        avatar(
            ui,
            rect,
            &item.id,
            &item.title,
            super::picture(item, pictures),
            palette,
        );
        ui.add_space(du(dim::PAD_CARD) - ui.spacing().item_spacing.x);
        let close = du(dim::ROW_COMPACT) + ui.spacing().item_spacing.x;
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - close);
            ui.spacing_mut().item_spacing.y = du(dim::PAD_SMALL);
            wrapped(ui, item.title.clone(), Type::ScreenTitle, palette.text_primary);
            chip_set(ui, |ui| {
                chip(
                    ui,
                    &text.source_label(&item.source),
                    ChipColor::Neutral,
                    false,
                    palette,
                );
                chip(
                    ui,
                    (text.section)(item.section),
                    ChipColor::Neutral,
                    false,
                    palette,
                );
                if item.is_orphan() {
                    chip(ui, text.badge_orphan, ChipColor::Warning, false, palette);
                }
                if item.usage == Usage::Running {
                    chip(ui, text.badge_running, ChipColor::Success, false, palette);
                }
            });
        });
        closed = IconButton::new(icon::CLOSE, text.close)
            .show(ui, palette)
            .clicked();
    });
    closed
}

fn facts(ui: &mut Ui, card: &Card<'_>, palette: &Palette) {
    let list = fact_list(card);
    property_list(ui, Id::unique(("facts", &card.item.id)), &list, palette);
}

/// The facts of the card, each when the entry has it.
fn fact_list<'a>(card: &Card<'a>) -> Vec<Property<'a>> {
    let item = card.item;
    let text = card.text;
    let installed = |at| format!("{} ({})", format::date(at, text), format::ago(at, card.now, text));
    let reason = |reason| match reason {
        Reason::Explicit => text.reason_explicit.to_owned(),
        Reason::Dependency => text.reason_dependency.to_owned(),
    };
    let rows = [
        (
            text.key_version,
            Some(item.version.clone()).filter(|version| !version.is_empty()),
        ),
        (
            text.key_package,
            Some(item.package.clone()).filter(|package| *package != item.title),
        ),
        (
            text.key_license,
            Some(item.licenses.join(", ")).filter(|list| !list.is_empty()),
        ),
        (text.key_developer, item.developer.clone()),
        (text.key_size, item.size.map(|size| format::size(size, text))),
        (text.key_installed, item.installed.map(installed)),
        (text.key_reason, item.reason.map(reason)),
        (
            text.key_launch,
            Some(format::usage_detail(item.usage, card.now, text)),
        ),
        (
            text.key_groups,
            Some(item.groups.join(", ")).filter(|list| !list.is_empty()),
        ),
        (text.key_removal, Some(removal_note(&item.removal, text))),
    ];
    rows.into_iter()
        .filter_map(|(label, value)| {
            value.map(|value| Property {
                label,
                value,
                copy: None,
            })
        })
        .collect()
}

/// The word for one kind of Docker object.
pub fn docker_part(object: DockerObject, text: &Text) -> &'static str {
    match object {
        DockerObject::Container => text.docker_part_container,
        DockerObject::Image => text.docker_part_image,
        DockerObject::Volume => text.docker_part_volume,
    }
}

/// What a Docker row removes, when it is more than the row itself.
fn docker_parts(ui: &mut Ui, card: &Card<'_>, palette: &Palette) {
    let Removal::Docker { parts } = &card.item.removal else {
        return;
    };
    if parts.len() < 2 {
        return;
    }
    let text = card.text;
    section_title(ui, text.heading_docker_parts, palette);
    let list: Vec<Property> = parts
        .iter()
        .map(|part| Property {
            label: docker_part(part.object, text),
            value: match part.size {
                Some(size) => format!("{} · {}", part.name, format::size(size, text)),
                None => part.name.clone(),
            },
            copy: None,
        })
        .collect();
    property_list(ui, Id::unique(("parts", &card.item.id)), &list, palette);
}

/// The command as a code block, or the reason there is none.
fn command_block(ui: &mut Ui, card: &Card<'_>, palette: &Palette) {
    section_title(ui, card.text.heading_command, palette);
    if let Removal::Unavailable { reason } = &card.item.removal {
        wrapped(ui, reason.clone(), Type::Body, palette.text_secondary);
        return;
    }
    let Some(command) = card.command else {
        wrapped(ui, card.text.needed_by_others, Type::Body, palette.text_secondary);
        return;
    };
    CodeBlock {
        id: Id::unique(("command", &card.item.id)),
        code: command,
        language: Some("sh"),
        wrap: true,
        copy: Some(card.text.copy),
        max_height: None,
    }
    .show(ui, palette);
}

pub fn show(ui: &mut Ui, card: &Card<'_>, pictures: &mut Pictures, palette: &Palette) -> Option<Action> {
    let mut action = None;
    ui.painter().rect_filled(ui.max_rect(), 0.0, palette.surface);
    ScrollArea::vertical()
        .id_salt("card")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::new()
                .inner_margin(Margin::same(du(dim::MARGIN_SCREEN) as i8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(du(dim::GUTTER), du(dim::PAD_CARD));
                    if title_block(ui, card, pictures, palette) {
                        action = Some(Action::Close);
                    }
                    if !card.item.summary.is_empty() {
                        wrapped(ui, card.item.summary.clone(), Type::Body, palette.text_primary);
                    }
                    if let Some(asked) = action_buttons(ui, card, palette) {
                        action = Some(asked);
                    }
                    sections(ui, card, &mut action, palette);
                });
        });
    action
}

/// Launch, Remove and the website, each when the entry has it.
fn action_buttons(ui: &mut Ui, card: &Card<'_>, palette: &Palette) -> Option<Action> {
    let item = card.item;
    let text = card.text;
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        if item.launch.is_some()
            && ActionButton::new(text.launch)
                .icon(icon::PLAY_ARROW)
                .look(card.launch)
                .show(ui, palette)
                .clicked()
        {
            action = Some(Action::Launch);
        }
        let removable = !matches!(item.removal, Removal::Unavailable { .. });
        let remove = ActionButton::new(text.remove)
            .icon(icon::DELETE)
            .dangerous(true)
            .enabled(removable)
            .show(ui, palette);
        if remove.clicked() {
            action = Some(Action::Remove);
        }
        if let Some(url) = &item.homepage
            && ActionButton::new(text.site)
                .icon(icon::OPEN_IN_NEW)
                .show(ui, palette)
                .clicked()
        {
            action = Some(Action::Open(url.clone()));
        }
    });
    action
}

/// Everything under the buttons: the facts, the description, the parts, the launchers, the
/// command and the links to other entries.
fn sections(ui: &mut Ui, card: &Card<'_>, action: &mut Option<Action>, palette: &Palette) {
    let item = card.item;
    let text = card.text;
    facts(ui, card, palette);
    if !item.description.is_empty() && item.description != item.summary {
        section_title(ui, text.heading_description, palette);
        wrapped(ui, item.description.clone(), Type::Body, palette.text_secondary);
    }
    docker_parts(ui, card, palette);
    if item.launchers.len() > 1 {
        section_title(ui, text.heading_launchers, palette);
        wrapped(ui, item.launchers.join(", "), Type::Body, palette.text_secondary);
    }
    command_block(ui, card, palette);
    let linked = [
        (&item.required_by, text.required_by),
        (&item.depends, text.depends_on),
    ];
    for (names, heading) in linked {
        if !names.is_empty() {
            section_title(ui, &heading(names.len()), palette);
            links(ui, names, action, text, palette);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::model::{FlatpakScope, PackageSystem};

    #[test]
    fn every_way_of_removal_has_its_own_note() {
        let text = Lang::En.text();
        let removals = [
            Removal::Package(PackageSystem::Pacman),
            Removal::Flatpak {
                app_id: String::new(),
                scope: FlatpakScope::User,
            },
            Removal::Steam {
                app_id: String::new(),
            },
            Removal::Cargo { name: String::new() },
            Removal::Docker { parts: Vec::new() },
            Removal::Manager {
                name: String::new(),
                argv: Vec::new(),
                access: Access::User,
            },
            Removal::Unavailable {
                reason: String::new(),
            },
            Removal::Files {
                paths: Vec::new(),
                access: Access::User,
            },
            Removal::Files {
                paths: Vec::new(),
                access: Access::Root,
            },
        ];
        let mut notes: Vec<String> = removals
            .iter()
            .map(|removal| removal_note(removal, text))
            .collect();
        assert!(notes[0].contains("pacman"), "{}", notes[0]);
        notes.sort_unstable();
        notes.dedup();
        assert_eq!(notes.len(), removals.len());
    }
}
