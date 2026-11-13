//! The card of one item: what Discover shows about a program, plus what cleaning needs.

use eframe::egui::{self, Frame, Id, Margin, ScrollArea, Sense, Ui, vec2};

use super::format;
use super::operator::button::ActionButton;
use super::operator::code::CodeBlock;
use super::operator::content::{ChipColor, Property, avatar, chip, chip_set, property_list};
use super::operator::icon;
use super::operator::layout::section_title;
use super::operator::state::Look;
use super::operator::tokens::{Palette, Type, dim, du, text};
use super::pictures::Pictures;
use crate::i18n::Text;
use crate::model::{Access, Item, Reason, Removal, Usage};

pub enum Action {
    Launch,
    Remove,
    Open(String),
    /// Go to another package by name.
    Follow(String),
}

/// How many neighbours are listed before the list is cut.
const LINK_LIMIT: usize = 60;

fn removal_note(item: &Item, text: &Text) -> String {
    let note = match &item.removal {
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

fn title_block(ui: &mut Ui, card: &Card<'_>, pictures: &mut Pictures, palette: &Palette) {
    let item = card.item;
    let text = card.text;
    ui.horizontal_top(|ui| {
        let side = du(16.0);
        let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        avatar(
            ui,
            rect,
            &item.id,
            &item.title,
            pictures.get(item.icon.as_deref()),
            palette,
        );
        ui.add_space(du(dim::PAD_CARD) - ui.spacing().item_spacing.x);
        ui.vertical(|ui| {
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
    });
}

fn facts(ui: &mut Ui, card: &Card<'_>, palette: &Palette) {
    let item = card.item;
    let text = card.text;
    let mut list = Vec::new();
    let plain = |label, value: String| Property {
        label,
        value,
        copy: None,
    };
    if !item.version.is_empty() {
        list.push(plain(text.key_version, item.version.clone()));
    }
    if item.package != item.title {
        list.push(plain(text.key_package, item.package.clone()));
    }
    if !item.licenses.is_empty() {
        list.push(plain(text.key_license, item.licenses.join(", ")));
    }
    if let Some(developer) = &item.developer {
        list.push(plain(text.key_developer, developer.clone()));
    }
    if let Some(size) = item.size {
        list.push(plain(text.key_size, format::size(size, text)));
    }
    if let Some(installed) = item.installed {
        let when = format!(
            "{} ({})",
            format::date(installed, text),
            format::ago(installed, card.now, text)
        );
        list.push(plain(text.key_installed, when));
    }
    if let Some(reason) = item.reason {
        let why = match reason {
            Reason::Explicit => text.reason_explicit,
            Reason::Dependency => text.reason_dependency,
        };
        list.push(plain(text.key_reason, why.to_owned()));
    }
    list.push(plain(
        text.key_launch,
        format::usage_detail(item.usage, card.now, text),
    ));
    if !item.groups.is_empty() {
        list.push(plain(text.key_groups, item.groups.join(", ")));
    }
    list.push(plain(text.key_removal, removal_note(item, text)));
    property_list(ui, Id::unique(("facts", &item.id)), &list, palette);
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
    let item = card.item;
    let text = card.text;
    let mut action = None;
    let removable = !matches!(item.removal, Removal::Unavailable { .. });
    ui.painter().rect_filled(ui.max_rect(), 0.0, palette.surface);
    ScrollArea::vertical()
        .id_salt("card")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::new()
                .inner_margin(Margin::same(du(dim::MARGIN_SCREEN) as i8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(du(dim::GUTTER), du(dim::PAD_CARD));
                    title_block(ui, card, pictures, palette);
                    if !item.summary.is_empty() {
                        wrapped(ui, item.summary.clone(), Type::Body, palette.text_primary);
                    }
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
                    facts(ui, card, palette);
                    if !item.description.is_empty() && item.description != item.summary {
                        section_title(ui, text.heading_description, palette);
                        wrapped(ui, item.description.clone(), Type::Body, palette.text_secondary);
                    }
                    if item.launchers.len() > 1 {
                        section_title(ui, text.heading_launchers, palette);
                        wrapped(ui, item.launchers.join(", "), Type::Body, palette.text_secondary);
                    }
                    command_block(ui, card, palette);
                    if !item.required_by.is_empty() {
                        section_title(ui, &(text.required_by)(item.required_by.len()), palette);
                        links(ui, &item.required_by, &mut action, text, palette);
                    }
                    if !item.depends.is_empty() {
                        section_title(ui, &(text.depends_on)(item.depends.len()), palette);
                        links(ui, &item.depends, &mut action, text, palette);
                    }
                });
        });
    action
}
