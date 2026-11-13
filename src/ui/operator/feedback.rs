//! Feedback components: `LinearProgress` (indeterminate), `Banner`, `EmptyState` and `Skeleton`;
//! `Toast` lives in `toast.rs`.

use eframe::egui::{self, Color32, Mesh, Rect, Sense, Shadow, Ui, WidgetInfo, pos2, vec2};
use operator_ui_tokens::motion;

use super::icon::{self, Icon, paint_icon};
use super::motion::smooth;
use super::state::BORDER_THICK;
use super::text::wrapped;
use super::tokens::{BOLD, Palette, Type, dim, du, secs, text, tint, weighted};

/// The drop shadow of menus, tooltips and toasts: `0 2 DU 6 DU` black at 35%.
pub fn overlay_shadow() -> Shadow {
    Shadow {
        offset: [0, du(2.0) as i8],
        blur: du(6.0) as u8,
        spread: 0,
        color: Color32::from_black_alpha(89),
    }
}

/// `LinearProgress`, indeterminate: a segment of 35% of the track sweeping once per `pending_lap`.
pub fn paint_indeterminate(ui: &Ui, rect: Rect, palette: &Palette) {
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, palette.surface_alt);
    painter.rect_filled(rect, 0.0, tint(palette.pending, 0.2));
    let time = ui.input(|input| input.time) as f32;
    let progress = smooth((time / secs(motion::PENDING_LAP_MS)).fract());
    let length = rect.width() * 0.35;
    let left = rect.left() - length + (rect.width() + length) * progress;
    let segment = Rect::from_x_y_ranges(left..=left + length, rect.y_range()).intersect(rect);
    if segment.is_positive() {
        painter.rect_filled(segment, 0.0, palette.pending);
    }
    ui.ctx().request_repaint();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Success,
    Warning,
    Error,
}

impl Tone {
    pub fn colour(self, palette: &Palette) -> Color32 {
        match self {
            Tone::Success => palette.success,
            Tone::Warning => palette.warning,
            Tone::Error => palette.error,
        }
    }

    pub(super) fn icon(self) -> Icon {
        match self {
            Tone::Success => icon::CHECK_CIRCLE,
            Tone::Warning => icon::WARNING,
            Tone::Error => icon::ERROR,
        }
    }
}

/// `Banner`: a persistent message across its container, with a tone, an icon and wrapped text.
pub fn banner(ui: &mut Ui, tone: Tone, title: Option<&str>, message: &str, palette: &Palette) {
    let colour = tone.colour(palette);
    let pad = du(dim::PAD_CARD);
    let icon_size = du(8.0);
    let gap = du(dim::GUTTER);
    let width = ui.available_width();
    let text_width = (width - 2.0 * pad - BORDER_THICK - icon_size - gap).max(du(10.0));
    let heading = title;
    let title = title.map(|title| {
        wrapped(
            ui,
            weighted(title, Type::Body, BOLD, palette.text_primary),
            text_width,
        )
    });
    let body = wrapped(ui, text(message, Type::Body, palette.text_primary), text_width);
    let first_line = Type::Body.line();
    let text_top = (icon_size - first_line) / 2.0;
    let text_height = title.as_ref().map_or(0.0, |title| title.size().y) + body.size().y;
    let height = 2.0 * pad + icon_size.max(text_top + text_height);
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let name = match title {
        Some(_) => format!("{} {message}", heading.unwrap_or_default()),
        None => message.to_owned(),
    };
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, &name));
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, palette.surface);
    painter.rect_filled(rect, 0.0, tint(colour, 0.15));
    painter.rect_filled(
        Rect::from_min_size(rect.min, vec2(BORDER_THICK, rect.height())),
        0.0,
        colour,
    );
    let icon_left = rect.left() + BORDER_THICK + pad;
    let icon_centre = pos2(icon_left + icon_size / 2.0, rect.top() + pad + icon_size / 2.0);
    paint_icon(ui, tone.icon(), icon_centre, icon_size, colour);
    let mut y = rect.top() + pad + text_top;
    let x = icon_left + icon_size + gap;
    if let Some(title) = title {
        let height = title.size().y;
        ui.painter().galley(pos2(x, y), title, palette.text_primary);
        y += height;
    }
    ui.painter().galley(pos2(x, y), body, palette.text_primary);
}

/// `EmptyState`: why an area is empty, centred in it.
pub fn empty_state(
    ui: &mut Ui,
    rect: Rect,
    glyph: Icon,
    title: &str,
    description: Option<&str>,
    compact: bool,
    palette: &Palette,
) {
    let (pad_y, icon_size) = if compact {
        (du(6.0), du(8.0))
    } else {
        (du(10.0), du(12.0))
    };
    let width = (rect.width() - 2.0 * du(if compact { 3.0 } else { 4.0 })).min(du(90.0));
    let title = wrapped(ui, text(title, Type::RowTitle, palette.text_primary), width);
    let description =
        description.map(|line| wrapped(ui, text(line, Type::Body, palette.text_secondary), width));
    let gap = du(dim::GUTTER);
    let total = icon_size + gap + title.size().y + description.as_ref().map_or(0.0, |line| line.size().y);
    let top = (rect.center().y - total / 2.0).max(rect.top() + pad_y);
    let centre = rect.center().x;
    paint_icon(
        ui,
        glyph,
        pos2(centre, top + icon_size / 2.0),
        icon_size,
        palette.text_disabled,
    );
    let mut y = top + icon_size + gap;
    let title_height = title.size().y;
    paint_centred_line(ui, centre, y, title, palette.text_primary);
    y += title_height;
    if let Some(line) = description {
        paint_centred_line(ui, centre, y, line, palette.text_secondary);
    }
}

fn paint_centred_line(ui: &Ui, centre: f32, top: f32, galley: std::sync::Arc<egui::Galley>, colour: Color32) {
    let corner = pos2(centre - galley.size().x / 2.0, top).round();
    ui.painter().galley(corner, galley, colour);
}

/// `Skeleton` block: one placeholder shape at `rect`, with the highlight sweeping over it once
/// per `skeleton_lap`. Components lay their blocks out in the shape of their own content.
pub fn paint_skeleton(ui: &Ui, rect: Rect, palette: &Palette) {
    let time = ui.input(|input| input.time) as f32;
    let phase = (time / secs(motion::SKELETON_LAP_MS)).fract();
    paint_shimmer(ui, rect, phase, palette);
    ui.ctx().request_repaint();
}

/// The `Skeleton` of a text line in `kind`: a bar of its font size, centred on `centre_y`.
pub fn skeleton_text_line(left: f32, centre_y: f32, width: f32, kind: Type) -> Rect {
    Rect::from_center_size(pos2(left + width / 2.0, centre_y), vec2(width, kind.size()))
}

/// How much of the room a placeholder line takes, varied from line to line so the shapes read as
/// text, the same in every frame.
pub fn skeleton_share(seed: usize) -> f32 {
    const SHARES: [f32; 5] = [0.7, 0.45, 0.85, 0.55, 0.65];
    SHARES[seed % SHARES.len()]
}

/// `surface_alt` with a soft band of `pressed` crossing it from right to left.
fn paint_shimmer(ui: &Ui, rect: Rect, phase: f32, palette: &Palette) {
    ui.painter().rect_filled(rect, 0.0, palette.surface_alt);
    let band = rect.width().max(du(40.0));
    let centre = rect.right() + band - (rect.width() + 2.0 * band) * phase;
    let stops = [
        (centre - band, palette.surface_alt),
        (centre, palette.pressed),
        (centre + band, palette.surface_alt),
    ];
    let mut mesh = Mesh::default();
    for (x, colour) in stops {
        let x = x.clamp(rect.left(), rect.right());
        mesh.colored_vertex(pos2(x, rect.top()), colour);
        mesh.colored_vertex(pos2(x, rect.bottom()), colour);
    }
    for quad in 0..2 {
        let base = quad * 2;
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 1, base + 3, base + 2);
    }
    ui.painter().add(mesh);
}
