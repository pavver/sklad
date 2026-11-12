//! Operator UI for egui: the parts of the design system sklad uses, ported from the
//! specifications in `operator-design-system/docs` (the reference implementation is Leptos only).
//!
//! Draft components of the design system, likely to change once they are finished there:
//! `DataTable`, `PropertyList` and `Toolbar`.

pub mod button;
pub mod code;
pub mod content;
pub mod data_table;
pub mod dialog;
pub mod feedback;
pub mod icon;
pub mod layout;
pub mod motion;
pub mod search_field;
pub mod segmented;
pub mod state;
pub mod text;
pub mod tokens;

use eframe::egui::{self, CornerRadius, FontId, Margin, Shadow, Stroke, TextStyle, style::ScrollStyle};
use operator_ui_tokens::motion as durations;

use state::BORDER;
use tokens::{Palette, Type, dim, du, secs};

/// Applies the palette to what egui draws itself: panels, text fields, tooltips, scroll bars.
pub fn apply(ctx: &egui::Context, palette: &Palette) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::monospace(Type::AppTitle.size())),
            (TextStyle::Body, FontId::monospace(Type::Body.size())),
            (TextStyle::Button, FontId::monospace(Type::Body.size())),
            (TextStyle::Small, FontId::monospace(Type::Caption.size())),
            (TextStyle::Monospace, FontId::monospace(Type::Console.size())),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(du(dim::GUTTER), du(dim::GUTTER));
        style.spacing.interact_size.y = du(dim::ROW_STANDARD);
        style.spacing.tooltip_width = du(75.0);
        style.spacing.menu_margin = Margin::symmetric(du(dim::GUTTER) as i8, du(dim::PAD_TIGHT) as i8);
        style.spacing.scroll = ScrollStyle {
            floating: true,
            bar_width: du(dim::GUTTER),
            floating_width: du(dim::PAD_TIGHT),
            floating_allocated_width: 0.0,
            handle_min_length: du(6.0),
            bar_inner_margin: 0.0,
            bar_outer_margin: 0.0,
            foreground_color: false,
            dormant_background_opacity: 0.0,
            active_background_opacity: 0.0,
            interact_background_opacity: 1.0,
            dormant_handle_opacity: 1.0,
            active_handle_opacity: 1.0,
            interact_handle_opacity: 1.0,
            ..ScrollStyle::floating()
        };
        style.interaction.selectable_labels = false;
        style.interaction.tooltip_delay = secs(durations::TOOLTIP_SHOW_DELAY_MS);
        style.animation_time = secs(durations::STATE_COLOR_MS);

        let visuals = &mut style.visuals;
        visuals.dark_mode = palette.dark;
        visuals.panel_fill = palette.background;
        visuals.window_fill = palette.surface_alt;
        visuals.faint_bg_color = palette.surface_alt;
        visuals.extreme_bg_color = palette.surface_alt;
        visuals.text_edit_bg_color = Some(palette.surface);
        visuals.hyperlink_color = palette.accent;
        visuals.warn_fg_color = palette.warning;
        visuals.error_fg_color = palette.error;
        visuals.override_text_color = Some(palette.text_primary);
        visuals.text_cursor.stroke = Stroke::new(du(dim::GAP_MICRO), palette.accent);
        visuals.window_corner_radius = CornerRadius::ZERO;
        visuals.menu_corner_radius = CornerRadius::ZERO;
        visuals.window_shadow = Shadow::NONE;
        visuals.popup_shadow = feedback::overlay_shadow();
        visuals.window_stroke = Stroke::new(BORDER, palette.border_strong);
        visuals.selection.bg_fill = palette.selection;
        visuals.selection.stroke = Stroke::new(BORDER, palette.accent);

        // The scroll bar takes its thumb from these: idle, over the thumb, dragged.
        let states = [
            (&mut visuals.widgets.noninteractive, palette.border_strong),
            (&mut visuals.widgets.inactive, palette.border_strong),
            (&mut visuals.widgets.hovered, palette.text_secondary),
            (&mut visuals.widgets.active, palette.accent),
            (&mut visuals.widgets.open, palette.accent),
        ];
        for (widget, fill) in states {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.bg_stroke = Stroke::new(BORDER, palette.border);
            widget.fg_stroke = Stroke::new(BORDER, palette.text_primary);
            widget.corner_radius = CornerRadius::ZERO;
            widget.expansion = 0.0;
        }
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(BORDER, palette.divider);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(BORDER, palette.text_primary);
    });
}
