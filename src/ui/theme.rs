//! Operator UI tokens: the Design Unit grid, both palettes, the type scale.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, RichText, Shadow, Stroke,
    TextStyle,
};

/// One Design Unit is four logical points.
pub const fn du(units: f32) -> f32 {
    units * 4.0
}

pub const BORDER: f32 = du(0.25);
pub const BORDER_THICK: f32 = du(0.5);
pub const GAP_MICRO: f32 = du(0.5);
pub const PAD_TIGHT: f32 = du(1.0);
pub const PAD_SMALL: f32 = du(1.5);
pub const GUTTER: f32 = du(2.0);
pub const PAD_CONTENT: f32 = du(2.5);
pub const PAD_CARD: f32 = du(3.0);
pub const MARGIN_SCREEN: f32 = du(4.0);
pub const SECTION_GAP: f32 = du(6.0);
pub const COMPACT_ROW: f32 = du(8.0);
pub const STANDARD_ROW: f32 = du(10.0);
pub const AVATAR_ROW: f32 = du(14.0);

/// `clamp(min, ratio * parent, max)`: the sizing rule of every large container.
pub fn clamp_width(parent: f32, min_du: f32, ratio: f32, max_du: f32) -> f32 {
    (parent * ratio).clamp(du(min_du), du(max_du))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub surface_alt: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub accent: Color32,
    pub pending: Color32,
    pub success: Color32,
    pub error: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_disabled: Color32,
    pub disabled_surface: Color32,
    pub disabled_border: Color32,
}

const fn hex(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

pub const DARK: Palette = Palette {
    background: hex(0x0D1117),
    surface: hex(0x161B22),
    surface_alt: hex(0x0F141C),
    border: hex(0x3E4A5C),
    border_strong: hex(0x8B949E),
    accent: hex(0x58A6FF),
    pending: hex(0xE3B341),
    success: hex(0x39D353),
    error: hex(0xFF7B72),
    text_primary: hex(0xF0F6FC),
    text_secondary: hex(0x8B949E),
    text_disabled: hex(0x484F58),
    disabled_surface: hex(0x10141A),
    disabled_border: hex(0x1F252E),
};

pub const LIGHT: Palette = Palette {
    background: hex(0xF6F8FA),
    surface: hex(0xFFFFFF),
    surface_alt: hex(0xF0F2F5),
    border: hex(0xB2C0CE),
    border_strong: hex(0x57606A),
    accent: hex(0x0969DA),
    pending: hex(0x9A6700),
    success: hex(0x1A7F37),
    error: hex(0xCF222E),
    text_primary: hex(0x24292F),
    text_secondary: hex(0x57606A),
    text_disabled: hex(0x8C959F),
    disabled_surface: hex(0xF3F4F6),
    disabled_border: hex(0xD9E0E7),
};

pub const TERMINAL_BACKGROUND: Color32 = hex(0x0C0C0C);
pub const TERMINAL_FOREGROUND: Color32 = hex(0xCCCCCC);

/// `colour.copy(alpha = ...)` of the specification.
pub fn tint(colour: Color32, alpha: f32) -> Color32 {
    colour.gamma_multiply(alpha)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Medium,
    Bold,
}

/// The type scale of the design system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    AppTitle,
    RowTitle,
    Body,
    Subtitle,
    Console,
}

impl Type {
    pub fn size(self) -> f32 {
        match self {
            Type::AppTitle => 20.0,
            Type::RowTitle => 15.0,
            Type::Body => 14.0,
            Type::Subtitle => 11.0,
            Type::Console => 12.0,
        }
    }

    fn line_height(self) -> f32 {
        match self {
            Type::AppTitle => 26.0,
            Type::RowTitle => 20.0,
            Type::Body => 18.0,
            Type::Subtitle => 14.0,
            Type::Console => 16.0,
        }
    }

    fn weight(self) -> Weight {
        match self {
            Type::AppTitle => Weight::Bold,
            Type::RowTitle => Weight::Medium,
            _ => Weight::Regular,
        }
    }
}

/// Text in a style of the scale. Cascadia is a variable font, so the weight is a real one.
pub fn styled(text: impl Into<String>, kind: Type, colour: Color32) -> RichText {
    weighted(text, kind, kind.weight(), colour)
}

pub fn weighted(text: impl Into<String>, kind: Type, weight: Weight, colour: Color32) -> RichText {
    let axis = match weight {
        Weight::Regular => 400.0,
        Weight::Medium => 500.0,
        Weight::Bold => 700.0,
    };
    RichText::new(text)
        .font(FontId::monospace(kind.size()))
        .line_height(Some(kind.line_height()))
        .color(colour)
        .variation("wght", axis)
}

/// Cascadia first; the default fonts stay behind it for scripts it lacks, like CJK program names.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let cascadia = FontData::from_static(include_bytes!("../../assets/CascadiaMono.ttf"));
    let phosphor = FontData::from_static(include_bytes!("../../assets/Phosphor.ttf"));
    fonts.font_data.insert("cascadia".to_owned(), Arc::new(cascadia));
    fonts.font_data.insert("phosphor".to_owned(), Arc::new(phosphor));
    for family in [FontFamily::Monospace, FontFamily::Proportional] {
        let chain = fonts.families.entry(family).or_default();
        chain.insert(0, "cascadia".to_owned());
        chain.push("phosphor".to_owned());
    }
    ctx.set_fonts(fonts);
}

/// Applies the palette: right angles, flat surfaces, no shadows.
pub fn apply(ctx: &egui::Context, palette: &Palette, dark: bool) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::monospace(Type::AppTitle.size())),
            (TextStyle::Body, FontId::monospace(Type::Body.size())),
            (TextStyle::Button, FontId::monospace(Type::Body.size())),
            (TextStyle::Small, FontId::monospace(Type::Subtitle.size())),
            (TextStyle::Monospace, FontId::monospace(Type::Console.size())),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(GUTTER, GUTTER);
        style.spacing.button_padding = egui::vec2(MARGIN_SCREEN, PAD_TIGHT);
        style.spacing.interact_size.y = STANDARD_ROW;
        style.interaction.selectable_labels = false;
        style.animation_time = 0.15;

        let visuals = &mut style.visuals;
        visuals.dark_mode = dark;
        visuals.panel_fill = palette.background;
        visuals.window_fill = palette.surface;
        visuals.faint_bg_color = palette.surface_alt;
        visuals.extreme_bg_color = palette.surface_alt;
        visuals.text_edit_bg_color = Some(palette.surface);
        visuals.hyperlink_color = palette.accent;
        visuals.warn_fg_color = palette.pending;
        visuals.error_fg_color = palette.error;
        visuals.override_text_color = Some(palette.text_primary);
        visuals.window_corner_radius = CornerRadius::ZERO;
        visuals.menu_corner_radius = CornerRadius::ZERO;
        visuals.window_shadow = Shadow::NONE;
        visuals.popup_shadow = Shadow::NONE;
        visuals.window_stroke = Stroke::new(BORDER, palette.border);
        visuals.selection.bg_fill = tint(palette.accent, 0.3);
        visuals.selection.stroke = Stroke::new(BORDER, palette.accent);

        let states = [
            (
                &mut visuals.widgets.noninteractive,
                palette.background,
                palette.border,
            ),
            (&mut visuals.widgets.inactive, palette.surface, palette.border),
            (
                &mut visuals.widgets.hovered,
                palette.surface,
                palette.border_strong,
            ),
            (
                &mut visuals.widgets.active,
                palette.surface,
                palette.border_strong,
            ),
            (&mut visuals.widgets.open, palette.surface, palette.border_strong),
        ];
        for (widget, fill, border) in states {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.bg_stroke = Stroke::new(BORDER, border);
            widget.fg_stroke = Stroke::new(BORDER, palette.text_primary);
            widget.corner_radius = CornerRadius::ZERO;
            widget.expansion = 0.0;
        }
    });
}
