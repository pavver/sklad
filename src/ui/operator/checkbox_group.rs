//! `CheckboxGroup` (`docs/components/checkbox-group.md`), synchronous use: a tile changes on
//! click, with no request behind it, so it never shows `Pending` or a result.
//!
//! Two agreed departures from the specification: each tile may carry a `Counter` before its
//! indicator box, and its icon may be a picture from the icon theme of the system instead of a
//! glyph, drawn in its own colours.

use eframe::egui::{self, CursorIcon, Id, Rect, Sense, TextureHandle, Ui, WidgetInfo, pos2, vec2};

use super::feedback::{paint_skeleton, skeleton_share, skeleton_text_line};
use super::icon::{Icon, paint_icon};
use super::layout::{Tone, counter_size, paint_counter};
use super::state::{
    ControlState, Look, Outline, hover_amount, paint_border, paint_focus_ring, paint_ripple, press_amount,
};
use super::text::{paint_aligned, truncated};
use super::tokens::{BOLD, MEDIUM, Palette, Type, dim, du, mix, weighted};

/// The size of the item icons. The specification also has `Medium` (`icon_large`), which sklad
/// does not use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconSize {
    /// `icon_small` (`4 DU`).
    #[default]
    Small,
    /// The tile height less `pad_tight` on each side (`10 DU`).
    Large,
}

impl IconSize {
    /// The side in DU.
    pub fn du(self) -> f32 {
        match self {
            IconSize::Small => dim::ICON_SMALL,
            IconSize::Large => dim::ROW_CONTROL - 2.0 * dim::PAD_TIGHT,
        }
    }
}

/// The icon of a tile.
#[derive(Clone)]
pub enum TileIcon {
    /// A single-colour glyph, tinted with the state of the tile.
    Glyph(Icon),
    /// A picture in its own colours.
    Picture(TextureHandle),
}

/// One option.
pub struct GroupItem<'a> {
    pub name: &'a str,
    pub icon: Option<TileIcon>,
    pub count: Option<usize>,
    pub selected: bool,
}

/// The gap between tiles, in DU.
const TILE_GAP: f32 = 0.75;

/// The tiles, `0.75 DU` apart, full width.
pub struct CheckboxGroup<'a> {
    pub id: Id,
    pub items: &'a [GroupItem<'a>],
    pub icon_size: IconSize,
    /// The largest number a counter shows in full.
    pub counter_max: usize,
}

/// The `Skeleton` of a group while its options load: `tiles` tiles, each with blocks where the
/// icon, the name, the counter and the indicator box will be.
pub fn skeleton(ui: &mut Ui, tiles: usize, icon_size: IconSize, palette: &Palette) {
    for index in 0..tiles {
        if index > 0 {
            ui.add_space(du(TILE_GAP));
        }
        let size = vec2(ui.available_width(), du(dim::ROW_CONTROL));
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            continue;
        }
        ui.painter().rect_filled(rect, 0.0, palette.surface);
        let pad = match icon_size {
            IconSize::Large => dim::PAD_TIGHT,
            IconSize::Small => dim::GUTTER,
        };
        let side = du(icon_size.du());
        let icon = Rect::from_min_size(
            pos2(rect.left() + du(pad), rect.center().y - side / 2.0),
            vec2(side, side),
        );
        paint_skeleton(ui, icon, palette);
        let boxed = du(dim::ICON_LARGE);
        let boxed = Rect::from_min_size(
            pos2(
                rect.right() - du(dim::GUTTER) - boxed,
                rect.center().y - boxed / 2.0,
            ),
            vec2(boxed, boxed),
        );
        paint_skeleton(ui, boxed, palette);
        let counter = counter_size(ui, 99, usize::MAX);
        let counter = Rect::from_min_size(
            pos2(
                boxed.left() - du(dim::GUTTER) - counter.x,
                rect.center().y - counter.y / 2.0,
            ),
            counter,
        );
        paint_skeleton(ui, counter, palette);
        let left = icon.right() + du(dim::GUTTER);
        let room = (counter.left() - du(dim::GUTTER) - left).max(0.0);
        let line = skeleton_text_line(left, rect.center().y, room * skeleton_share(index), Type::Body);
        paint_skeleton(ui, line, palette);
        paint_border(
            ui,
            rect,
            Outline::of(ControlState::Idle, palette),
            &Look::IDLE,
            palette,
        );
    }
}

/// A glyph tinted with the state of the tile, or a picture in its own colours.
fn paint_tile_icon(
    ui: &Ui,
    icon: &TileIcon,
    centre: egui::Pos2,
    side: f32,
    selected: bool,
    palette: &Palette,
) {
    match icon {
        TileIcon::Glyph(glyph) => {
            let colour = if selected { palette.accent } else { palette.icon };
            paint_icon(ui, *glyph, centre, side, colour);
        }
        TileIcon::Picture(texture) => {
            let place = Rect::from_center_size(centre, vec2(side, side));
            egui::Image::from_texture(texture)
                .fit_to_exact_size(place.size())
                .maintain_aspect_ratio(true)
                .paint_at(ui, place);
        }
    }
}

/// The indicator box at the end of the tile, checked when the tile is selected. Returns where
/// it lies.
fn paint_indicator(ui: &Ui, tile: Rect, selected: bool, palette: &Palette) -> Rect {
    let side = du(dim::ICON_LARGE);
    let corner = pos2(
        tile.right() - du(dim::GUTTER) - side,
        tile.center().y - side / 2.0,
    );
    let boxed = Rect::from_min_size(corner, vec2(side, side));
    let state = if selected {
        ui.painter()
            .rect_filled(boxed, 0.0, mix(palette.surface, palette.accent, 0.15));
        paint_icon(
            ui,
            super::icon::CHECK,
            boxed.center(),
            du(dim::ICON_SMALL),
            palette.accent,
        );
        ControlState::Selected
    } else {
        ControlState::Idle
    };
    let mut outline = Outline::of(state, palette);
    outline.width = super::state::BORDER;
    paint_border(ui, boxed, outline, &Look::IDLE, palette);
    boxed
}

impl CheckboxGroup<'_> {
    /// Returns the index of the tile clicked.
    pub fn show(&self, ui: &mut Ui, palette: &Palette) -> Option<usize> {
        let mut clicked = None;
        for (index, item) in self.items.iter().enumerate() {
            if index > 0 {
                ui.add_space(du(TILE_GAP));
            }
            if self.tile(ui, index, item, palette) {
                clicked = Some(index);
            }
        }
        clicked
    }

    fn tile(&self, ui: &mut Ui, index: usize, item: &GroupItem<'_>, palette: &Palette) -> bool {
        let size = vec2(ui.available_width(), du(dim::ROW_CONTROL));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let response = ui
            .interact(rect, self.id.with(index), Sense::click())
            .union(response);
        let selected = item.selected;
        response.widget_info(|| WidgetInfo::selected(egui::Role::CheckBox, true, selected, item.name));
        if ui.is_rect_visible(rect) {
            let hover = hover_amount(ui, &response);
            let press = press_amount(ui, &response);
            let rest = if selected {
                mix(palette.surface, palette.accent, 0.15)
            } else {
                mix(palette.surface, palette.hover, hover)
            };
            ui.painter()
                .rect_filled(rect, 0.0, mix(rest, palette.pressed, press));
            paint_ripple(ui, &response, rect, true, palette);

            let large = self.icon_size == IconSize::Large && item.icon.is_some();
            let pad = if large { dim::PAD_TIGHT } else { dim::GUTTER };
            let mut left = rect.left() + du(pad);
            let ink = if selected {
                palette.accent
            } else {
                palette.text_primary
            };
            if let Some(icon) = &item.icon {
                let side = du(self.icon_size.du());
                paint_tile_icon(
                    ui,
                    icon,
                    pos2(left + side / 2.0, rect.center().y),
                    side,
                    selected,
                    palette,
                );
                left += side + du(dim::GUTTER);
            }
            let boxed = paint_indicator(ui, rect, selected, palette);
            let mut right = boxed.left() - du(dim::GUTTER);
            if let Some(count) = item.count {
                let size = counter_size(ui, count, self.counter_max);
                let badge = Rect::from_min_size(pos2(right - size.x, rect.center().y - size.y / 2.0), size);
                paint_counter(ui, badge, count, self.counter_max, Tone::Neutral, palette);
                right = badge.left() - du(dim::GUTTER);
            }

            let weight = if selected { BOLD } else { MEDIUM };
            let name = truncated(
                ui,
                weighted(item.name, Type::Body, weight, ink),
                (right - left).max(0.0),
            );
            let name_rect = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom()));
            paint_aligned(ui, name_rect, name, egui::Align::Min, ink);

            let state = if response.is_pointer_button_down_on() {
                ControlState::Pressed
            } else if selected {
                ControlState::Selected
            } else {
                ControlState::Idle
            };
            paint_border(ui, rect, Outline::of(state, palette), &Look::IDLE, palette);
            paint_focus_ring(ui, &response, rect, palette);
        }
        response.on_hover_cursor(CursorIcon::PointingHand).clicked()
    }
}
