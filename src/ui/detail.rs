//! The card of one item: what Discover shows about a program, plus what cleaning needs.

use eframe::egui::{self, Rect, RichText, ScrollArea, Sense, Ui, vec2};

use super::format;
use super::glyph;
use super::theme::{GUTTER, PAD_CARD, PAD_SMALL, Palette, SECTION_GAP, Type, Weight, du, styled, weighted};
use super::widgets::{IconCache, Tone, action_button, avatar, badge, chip, copy_button};
use crate::i18n::Text;
use crate::model::{Access, Item, Reason, Removal, Usage};

pub enum Action {
    Remove,
    Open(String),
    /// Go to another package by name.
    Follow(String),
}

/// How many neighbours are listed before the list is cut.
const LINK_LIMIT: usize = 60;
/// Wide enough for the longest label of the card.
const KEY_COLUMN: f32 = du(23.0);

fn fact(ui: &mut Ui, key: &str, value: impl Into<String>, palette: &Palette) {
    ui.vertical(|ui| {
        ui.set_min_width(KEY_COLUMN);
        ui.add(egui::Label::new(styled(key, Type::Console, palette.text_secondary)).extend());
    });
    ui.add(egui::Label::new(styled(value, Type::Console, palette.text_primary)).wrap());
    ui.end_row();
}

fn heading(ui: &mut Ui, text: &str, palette: &Palette) {
    ui.add_space(PAD_SMALL);
    ui.label(weighted(text, Type::Body, Weight::Bold, palette.text_primary));
}

fn removal_note(item: &Item, text: &Text) -> &'static str {
    match &item.removal {
        Removal::Pacman => text.by_pacman,
        Removal::Flatpak { .. } => "Flatpak",
        Removal::Steam { .. } => text.by_steam,
        Removal::Cargo { .. } => "cargo uninstall",
        Removal::Manager { .. } => text.by_language_manager,
        Removal::Unavailable { .. } => text.not_removable_here,
        Removal::Files {
            access: Access::User, ..
        } => text.files_to_bin,
        Removal::Files {
            access: Access::Root, ..
        } => text.files_for_good,
    }
}

fn links(ui: &mut Ui, names: &[String], action: &mut Option<Action>, text: &Text, palette: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(PAD_SMALL, PAD_SMALL);
        ui.spacing_mut().interact_size.y = du(6.0);
        for name in names.iter().take(LINK_LIMIT) {
            if chip(ui, name, palette).clicked() {
                *action = Some(Action::Follow(name.clone()));
            }
        }
        if names.len() > LINK_LIMIT {
            let rest = (text.and_more)(names.len() - LINK_LIMIT);
            ui.label(styled(rest, Type::Subtitle, palette.text_secondary));
        }
    });
}

pub struct Card<'a> {
    pub item: &'a Item,
    pub command: Option<&'a str>,
    pub now: i64,
    pub text: &'static Text,
}

fn title_block(ui: &mut Ui, card: &Card<'_>, icons: &mut IconCache, palette: &Palette) {
    let item = card.item;
    let text = card.text;
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(du(16.0), du(16.0)), Sense::hover());
        let rect: Rect = rect;
        avatar(ui, rect, item.icon.as_deref(), &item.title, icons, palette);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = PAD_SMALL;
            ui.add(egui::Label::new(styled(item.title.clone(), Type::AppTitle, palette.text_primary)).wrap());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(PAD_SMALL, PAD_SMALL);
                ui.spacing_mut().interact_size.y = du(5.0);
                badge(ui, &text.source_label(&item.source), palette.accent);
                badge(ui, (text.section)(item.section), palette.text_secondary);
                if item.is_orphan() {
                    badge(ui, text.badge_orphan, palette.pending);
                }
                if item.usage == Usage::Running {
                    badge(ui, text.badge_running, palette.success);
                }
            });
        });
    });
}

fn facts(ui: &mut Ui, card: &Card<'_>, palette: &Palette) {
    let item = card.item;
    let text = card.text;
    ui.spacing_mut().interact_size.y = 0.0;
    egui::Grid::new("facts")
        .num_columns(2)
        .spacing(vec2(PAD_CARD, PAD_SMALL))
        .show(ui, |ui| {
            if !item.version.is_empty() {
                fact(ui, text.key_version, item.version.clone(), palette);
            }
            if item.package != item.title {
                fact(ui, text.key_package, item.package.clone(), palette);
            }
            if !item.licenses.is_empty() {
                fact(ui, text.key_license, item.licenses.join(", "), palette);
            }
            if let Some(developer) = &item.developer {
                fact(ui, text.key_developer, developer.clone(), palette);
            }
            if let Some(size) = item.size {
                fact(ui, text.key_size, format::size(size, text), palette);
            }
            if let Some(installed) = item.installed {
                let when = format!(
                    "{} ({})",
                    format::date(installed, text),
                    format::ago(installed, card.now, text)
                );
                fact(ui, text.key_installed, when, palette);
            }
            if let Some(reason) = item.reason {
                let why = match reason {
                    Reason::Explicit => text.reason_explicit,
                    Reason::Dependency => text.reason_dependency,
                };
                fact(ui, text.key_reason, why, palette);
            }
            fact(
                ui,
                text.key_launch,
                format::usage_detail(item.usage, card.now, text),
                palette,
            );
            if !item.groups.is_empty() {
                fact(ui, text.key_groups, item.groups.join(", "), palette);
            }
            fact(ui, text.key_removal, removal_note(item, text), palette);
        });
}

/// The command as a sunken block, or the reason there is none.
fn command_block(ui: &mut Ui, card: &Card<'_>, palette: &Palette) {
    heading(ui, card.text.heading_command, palette);
    if let Removal::Unavailable { reason } = &card.item.removal {
        ui.add(egui::Label::new(styled(reason.clone(), Type::Subtitle, palette.text_secondary)).wrap());
        return;
    }
    let Some(command) = card.command else {
        let note = card.text.needed_by_others;
        ui.add(egui::Label::new(styled(note, Type::Subtitle, palette.text_secondary)).wrap());
        return;
    };
    super::dialog::terminal(ui, |ui| {
        ui.add(
            egui::Label::new(
                RichText::new(command)
                    .monospace()
                    .color(super::theme::TERMINAL_FOREGROUND),
            )
            .wrap(),
        );
    });
    copy_button(ui, card.text.copy, command, palette);
}

pub fn show(ui: &mut Ui, card: &Card<'_>, icons: &mut IconCache, palette: &Palette) -> Option<Action> {
    let item = card.item;
    let text = card.text;
    let mut action = None;
    let removable = !matches!(item.removal, Removal::Unavailable { .. });
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = GUTTER;
        title_block(ui, card, icons, palette);
        if !item.summary.is_empty() {
            ui.add(egui::Label::new(styled(item.summary.clone(), Type::Body, palette.text_primary)).wrap());
        }
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                text.remove,
                Some(glyph::TRASH),
                Tone::Danger,
                removable,
                palette,
            )
            .clicked()
            {
                action = Some(Action::Remove);
            }
            if let Some(url) = &item.homepage
                && action_button(ui, text.site, Some(glyph::GLOBE), Tone::Plain, true, palette).clicked()
            {
                action = Some(Action::Open(url.clone()));
            }
        });
        ui.add_space(PAD_SMALL);
        facts(ui, card, palette);
        if !item.description.is_empty() && item.description != item.summary {
            heading(ui, text.heading_description, palette);
            ui.add(
                egui::Label::new(styled(
                    item.description.clone(),
                    Type::Body,
                    palette.text_secondary,
                ))
                .wrap(),
            );
        }
        if item.launchers.len() > 1 {
            heading(ui, text.heading_launchers, palette);
            ui.add(
                egui::Label::new(styled(
                    item.launchers.join(", "),
                    Type::Console,
                    palette.text_secondary,
                ))
                .wrap(),
            );
        }
        command_block(ui, card, palette);
        if !item.required_by.is_empty() {
            heading(ui, &(text.required_by)(item.required_by.len()), palette);
            links(ui, &item.required_by, &mut action, text, palette);
        }
        if !item.depends.is_empty() {
            heading(ui, &(text.depends_on)(item.depends.len()), palette);
            links(ui, &item.depends, &mut action, text, palette);
        }
        ui.add_space(SECTION_GAP);
    });
    action
}
