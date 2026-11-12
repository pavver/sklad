//! The tokens of Operator UI in egui terms: Design Units, the palette, the type scale, the font.
//! Values come from the `operator-ui-tokens` crate; nothing here defines a value of its own.

use std::sync::Arc;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, RichText};
use operator_ui_tokens as tokens;
pub use tokens::dim;
use tokens::{DU_POINTS, Rgba, TU_POINTS, TypeStyle};

/// Design Units to logical points: `1 DU = 4 points`.
pub const fn du(units: f32) -> f32 {
    units * DU_POINTS
}

/// Type Units to logical points. sklad has no text scale setting, so `1 TU = 1 point`.
pub const fn tu(units: f32) -> f32 {
    units * TU_POINTS
}

/// Milliseconds of a motion token in seconds.
pub fn secs(ms: u32) -> f32 {
    ms as f32 / 1000.0
}

const fn colour(value: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied_const(value.r, value.g, value.b, value.a)
}

macro_rules! palette {
    ($($name:ident),* $(,)?) => {
        /// One palette of the Operator theme with egui colours.
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct Palette {
            pub dark: bool,
            $(pub $name: Color32,)*
        }

        impl Palette {
            const fn of(dark: bool) -> Self {
                let source = tokens::Theme::Operator.palette(dark);
                Self { dark, $($name: colour(source.$name),)* }
            }
        }
    };
}

palette!(
    background,
    surface,
    surface_alt,
    border,
    border_strong,
    divider,
    text_primary,
    text_secondary,
    text_disabled,
    icon,
    icon_secondary,
    icon_disabled,
    accent,
    accent_muted,
    on_accent,
    success,
    warning,
    error,
    info,
    pending,
    hover,
    pressed,
    selection,
    ripple,
    disabled_surface,
    disabled_border,
    shadow,
    status_online,
    status_offline,
    status_alarm,
    status_error,
    status_activity,
    status_border,
    status_fallback,
    toggle_day_sky,
    toggle_night_sky,
    toggle_sun,
    toggle_moon,
    toggle_crater,
    toggle_cloud,
    toggle_cloud_back,
    toggle_star,
    term_background,
    term_foreground,
    term_black,
    term_red,
    term_green,
    term_yellow,
    term_blue,
    term_magenta,
    term_cyan,
    term_white,
    term_bright_black,
    term_bright_red,
    term_bright_green,
    term_bright_yellow,
    term_bright_blue,
    term_bright_magenta,
    term_bright_cyan,
    term_bright_white,
    syntax_keyword,
    syntax_string,
    syntax_number,
    syntax_comment,
    syntax_type,
    syntax_function,
    syntax_constant,
    syntax_property,
    syntax_meta,
    syntax_punctuation,
);

pub const DARK: Palette = Palette::of(true);
pub const LIGHT: Palette = Palette::of(false);

impl Palette {
    /// The sixteen `term_*` colours in SGR order: the eight normal ones, then the bright ones.
    pub fn terminal(&self) -> [Color32; 16] {
        [
            self.term_black,
            self.term_red,
            self.term_green,
            self.term_yellow,
            self.term_blue,
            self.term_magenta,
            self.term_cyan,
            self.term_white,
            self.term_bright_black,
            self.term_bright_red,
            self.term_bright_green,
            self.term_bright_yellow,
            self.term_bright_blue,
            self.term_bright_magenta,
            self.term_bright_cyan,
            self.term_bright_white,
        ]
    }
}

/// A colour at a share of its opacity, composited over whatever lies below: the tinted backgrounds.
pub fn tint(colour: Color32, alpha: f32) -> Color32 {
    colour.gamma_multiply(alpha.clamp(0.0, 1.0))
}

/// A blend of two colours, `amount` 0 giving the first.
pub fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    from.lerp_to_gamma(to, amount.clamp(0.0, 1.0))
}

/// The type scale of the design system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    AppTitle,
    ScreenTitle,
    SectionTitle,
    RowTitle,
    Body,
    BodySmall,
    Caption,
    Console,
}

impl Type {
    fn style(self) -> TypeStyle {
        use tokens::typography as t;
        match self {
            Type::AppTitle => t::APP_TITLE,
            Type::ScreenTitle => t::SCREEN_TITLE,
            Type::SectionTitle => t::SECTION_TITLE,
            Type::RowTitle => t::ROW_TITLE,
            Type::Body => t::BODY,
            Type::BodySmall => t::BODY_SMALL,
            Type::Caption => t::CAPTION,
            Type::Console => t::CONSOLE,
        }
    }

    pub fn size(self) -> f32 {
        tu(self.style().size)
    }

    pub fn line(self) -> f32 {
        tu(self.style().line)
    }

    pub fn weight(self) -> u16 {
        self.style().weight
    }

    pub fn font(self) -> FontId {
        FontId::monospace(self.size())
    }
}

/// The weights in use besides the regular one of each style: medium, bold.
pub const MEDIUM: u16 = 500;
pub const BOLD: u16 = 700;

/// Text in a style of the scale, with the weight of the style.
pub fn text(content: impl Into<String>, kind: Type, colour: Color32) -> RichText {
    weighted(content, kind, kind.weight(), colour)
}

/// Text in a style of the scale with another weight. Cascadia Code is variable, so it is a real one.
pub fn weighted(content: impl Into<String>, kind: Type, weight: u16, colour: Color32) -> RichText {
    RichText::new(content)
        .font(kind.font())
        .line_height(Some(kind.line()))
        .color(colour)
        .variation("wght", f32::from(weight))
}

/// Cascadia Code first; egui's own fonts stay behind it for scripts it lacks, like CJK names.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let cascadia = FontData::from_static(include_bytes!("../../../assets/CascadiaCode.ttf"));
    fonts.font_data.insert("cascadia".to_owned(), Arc::new(cascadia));
    for family in [FontFamily::Monospace, FontFamily::Proportional] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "cascadia".to_owned());
    }
    ctx.set_fonts(fonts);
}
