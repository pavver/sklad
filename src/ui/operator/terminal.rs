//! `ConsoleTerminal` with xterm SGR colours (`docs/components/console-terminal.md`).

use eframe::egui::{self, Color32, Id, ScrollArea, Sense, Stroke, StrokeKind, TextFormat, Ui, vec2};
use eframe::epaint::text::VariationCoords;

use super::code::{format, job};
use super::state::BORDER;
use super::tokens::{BOLD, Palette, dim, du};

/// The text style a log carries from one escape sequence to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Style {
    foreground: Option<Color32>,
    background: Option<Color32>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    inverse: bool,
}

fn xterm_256(index: u16, palette: &Palette) -> Color32 {
    match index {
        0..=15 => palette.terminal()[index as usize],
        16..=231 => {
            let n = index - 16;
            let level = |v: u16| if v == 0 { 0 } else { (55 + v * 40) as u8 };
            Color32::from_rgb(level(n / 36), level((n / 6) % 6), level(n % 6))
        }
        _ => {
            let grey = (8 + (index.min(255) - 232) * 10) as u8;
            Color32::from_rgb(grey, grey, grey)
        }
    }
}

/// Reads an extended colour (`5;n` or `2;r;g;b`) from the parameters after `38` or `48`.
fn extended(params: &[u16], palette: &Palette) -> (Option<Color32>, usize) {
    match params.first() {
        Some(5) => (params.get(1).map(|&n| xterm_256(n, palette)), 2),
        Some(2) => match params.get(1..4) {
            Some([r, g, b]) => (Some(Color32::from_rgb(*r as u8, *g as u8, *b as u8)), 4),
            _ => (None, params.len()),
        },
        _ => (None, 1),
    }
}

/// What an SGR code does to the style.
type Apply = fn(&mut Style);

/// SGR codes that turn one attribute on or off, or reset the whole style.
const ATTRIBUTES: [(u16, Apply); 14] = [
    (0, |style| *style = Style::default()),
    (1, |style| style.bold = true),
    (2, |style| style.dim = true),
    (3, |style| style.italic = true),
    (4, |style| style.underline = true),
    (7, |style| style.inverse = true),
    (9, |style| style.strike = true),
    (22, |style| (style.bold, style.dim) = (false, false)),
    (23, |style| style.italic = false),
    (24, |style| style.underline = false),
    (27, |style| style.inverse = false),
    (29, |style| style.strike = false),
    (39, |style| style.foreground = None),
    (49, |style| style.background = None),
];

/// Which colour a basic colour code sets, the foreground or not, and its index in the terminal
/// palette.
fn basic_colour(code: u16) -> Option<(bool, usize)> {
    let (foreground, index) = match code {
        30..=37 => (true, code - 30),
        40..=47 => (false, code - 40),
        90..=97 => (true, code - 90 + 8),
        100..=107 => (false, code - 100 + 8),
        _ => return None,
    };
    Some((foreground, usize::from(index)))
}

fn set_colour(style: &mut Style, foreground: bool, colour: Option<Color32>) {
    if foreground {
        style.foreground = colour;
    } else {
        style.background = colour;
    }
}

/// Applies one SGR code, reading what follows it when it needs that. Returns how many of the
/// following parameters it used.
fn apply_code(style: &mut Style, code: u16, rest: &[u16], palette: &Palette) -> usize {
    if code == 38 || code == 48 {
        let (colour, used) = extended(rest, palette);
        set_colour(style, code == 38, colour);
        return used;
    }
    if let Some((foreground, index)) = basic_colour(code) {
        set_colour(style, foreground, Some(palette.terminal()[index]));
    } else if let Some((_, apply)) = ATTRIBUTES.iter().find(|(known, _)| *known == code) {
        apply(style);
    }
    0
}

fn apply_sgr(style: &mut Style, params: &[u16], palette: &Palette) {
    if params.is_empty() {
        *style = Style::default();
    }
    let mut next = 0;
    while let Some(&code) = params.get(next) {
        next += 1 + apply_code(style, code, &params[next + 1..], palette);
    }
}

/// One line of output as styled pieces.
type Line = Vec<(Style, String)>;

/// Turns log entries into lines: SGR sets the style, other escapes are dropped, a lone carriage
/// return rewrites the line.
fn parse_logs(logs: &[String], palette: &Palette) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut style = Style::default();
    for entry in logs {
        let mut line: Line = Vec::new();
        let mut chars = entry.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\u{1b}' => escape(&mut chars, &mut style, palette),
                '\n' => lines.push(std::mem::take(&mut line)),
                '\r' if chars.peek() == Some(&'\n') => {}
                '\r' => line.clear(),
                _ => push_char(&mut line, style, c),
            }
        }
        lines.push(line);
    }
    lines
}

type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

/// Reads an escape sequence after its `ESC`: SGR changes `style`, the rest is skipped.
fn escape(chars: &mut Chars<'_>, style: &mut Style, palette: &Palette) {
    match chars.next() {
        Some('[') => {
            if let Some(params) = sgr_params(chars) {
                apply_sgr(style, &params, palette);
            }
        }
        Some(']') => skip_osc(chars),
        Some('(' | ')') => {
            chars.next();
        }
        _ => {}
    }
}

/// Reads a control sequence after `ESC [` and returns its parameters when it is an SGR, the
/// one that ends in `m`.
fn sgr_params(chars: &mut Chars<'_>) -> Option<Vec<u16>> {
    let mut body = String::new();
    let last = chars.find(|next| {
        let done = ('@'..='~').contains(next);
        if !done {
            body.push(*next);
        }
        done
    });
    (last == Some('m')).then(|| {
        body.split([';', ':'])
            .filter_map(|part| part.parse().ok())
            .collect()
    })
}

/// Skips an operating system command after `ESC ]`, up to BEL or `ESC \`.
fn skip_osc(chars: &mut Chars<'_>) {
    while let Some(next) = chars.next() {
        if next == '\u{7}' || (next == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
            break;
        }
    }
}

/// Adds `c` to the line, to the last piece while the style holds.
fn push_char(line: &mut Line, style: Style, c: char) {
    match line.last_mut() {
        Some((last, piece)) if *last == style => piece.push(c),
        _ => line.push((style, c.to_string())),
    }
}

fn style_format(style: Style, palette: &Palette) -> TextFormat {
    let mut foreground = style.foreground.unwrap_or(palette.term_foreground);
    let mut background = style.background.unwrap_or(Color32::TRANSPARENT);
    if style.inverse {
        let back = style.background.unwrap_or(palette.term_background);
        background = foreground;
        foreground = back;
    }
    if style.dim {
        foreground = foreground.gamma_multiply(0.6);
    }
    let mut look = format(foreground);
    look.background = background;
    look.italics = style.italic;
    if style.bold {
        look.coords = VariationCoords::new([(b"wght", f32::from(BOLD))]);
    }
    if style.underline {
        look.underline = Stroke::new(1.0, foreground);
    }
    if style.strike {
        look.strikethrough = Stroke::new(1.0, foreground);
    }
    look
}

/// `ConsoleTerminal`: a log view in the `term_*` palette that follows the newest line.
pub fn console_terminal(ui: &mut Ui, id: Id, logs: &[String], height: f32, palette: &Palette) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, du(height)), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.term_background);
    let pad = du(dim::GUTTER);
    let inner = rect.shrink(pad);
    let lines = parse_logs(logs, palette);
    let mut job = job(Some(inner.width()));
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            job.append("\n", 0.0, format(palette.term_foreground));
        }
        for (style, piece) in line {
            job.append(piece, 0.0, style_format(*style, palette));
        }
    }
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ScrollArea::vertical()
            .id_salt(id)
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(egui::Label::new(job).wrap());
            });
    });
    ui.painter()
        .rect_stroke(rect, 0.0, Stroke::new(BORDER, palette.border), StrokeKind::Inside);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::operator::tokens::DARK;

    #[test]
    fn escapes_colour_the_text_and_vanish() {
        let logs = ["\u{1b}[1;31merror:\u{1b}[0m failed".to_owned()];
        let lines = parse_logs(&logs, &DARK);
        assert_eq!(lines.len(), 1);
        let (style, text) = &lines[0][0];
        assert_eq!(text, "error:");
        assert!(style.bold);
        assert_eq!(style.foreground, Some(DARK.term_red));
        assert_eq!(lines[0][1].1, " failed");
    }

    /// The style of each piece of the first line of `log`, with its text. Neighbours of one
    /// style join into one piece.
    fn styled(log: &str) -> Vec<(Style, String)> {
        parse_logs(&[log.to_owned()], &DARK).swap_remove(0)
    }

    /// The style `letter` is drawn in.
    fn style_of(pieces: &[(Style, String)], letter: char) -> Option<Style> {
        pieces
            .iter()
            .find(|(_, piece)| piece.contains(letter))
            .map(|(style, _)| *style)
    }

    #[test]
    fn colours_come_in_every_form_a_terminal_knows() {
        let pieces =
            styled("\u{1b}[38;5;196ma\u{1b}[48;2;1;2;3mb\u{1b}[39;49;94mc\u{1b}[42;105md\u{1b}[38;5;244me");
        assert_eq!(
            pieces[0].0.foreground,
            Some(Color32::from_rgb(255, 0, 0)),
            "xterm cube"
        );
        assert_eq!(
            pieces[1].0.background,
            Some(Color32::from_rgb(1, 2, 3)),
            "true colour"
        );
        assert_eq!(
            pieces[2].0.foreground,
            Some(DARK.terminal()[12]),
            "bright foreground"
        );
        assert_eq!(pieces[2].0.background, None, "49 resets the background");
        assert_eq!(
            pieces[3].0.background,
            Some(DARK.terminal()[13]),
            "the last of 42;105"
        );
        assert_eq!(
            pieces[4].0.foreground,
            Some(Color32::from_rgb(128, 128, 128)),
            "xterm grey"
        );
        assert_eq!(
            xterm_256(3, &DARK),
            DARK.terminal()[3],
            "the first 16 are the palette"
        );
        assert_eq!(xterm_256(16, &DARK), Color32::from_rgb(0, 0, 0));
    }

    #[test]
    fn attributes_turn_on_and_off() {
        let pieces = styled("\u{1b}[1;2;3;4;7;9ma\u{1b}[22;23;24;27;29mb\u{1b}[31m\u{1b}[mc");
        let on = style_of(&pieces, 'a').unwrap_or_default();
        assert!(on.bold && on.dim && on.italic && on.underline && on.inverse && on.strike);
        assert_eq!(
            style_of(&pieces, 'b'),
            Some(Style::default()),
            "each attribute has its reset"
        );
        assert_eq!(
            style_of(&pieces, 'c'),
            Some(Style::default()),
            "an empty SGR resets everything"
        );
    }

    #[test]
    fn a_broken_extended_colour_takes_no_colour() {
        let pieces = styled("\u{1b}[31;38;2;1ma\u{1b}[31;38;7;4mb\u{1b}[31;38;5mc");
        let foreground = |letter| style_of(&pieces, letter).and_then(|style| style.foreground);
        assert_eq!(foreground('a'), None, "too few parts for true colour");
        assert_eq!(foreground('b'), None, "an unknown colour form");
        assert_eq!(
            style_of(&pieces, 'b').map(|style| style.underline),
            Some(true),
            "and what follows it still counts"
        );
        assert_eq!(foreground('c'), None, "no index after 5");
    }

    #[test]
    fn other_escapes_vanish_without_a_trace() {
        let pieces = styled("\u{1b}]0;title\u{7}a\u{1b}]8;;x\u{1b}\\b\u{1b}(Bc\u{1b}[2Kd\u{1b}7e");
        let text: String = pieces.iter().map(|(_, piece)| piece.as_str()).collect();
        assert_eq!(text, "abcde");
        assert!(pieces.iter().all(|(style, _)| *style == Style::default()));
        let lines = parse_logs(&["one\r\ntwo".to_owned()], &DARK);
        assert_eq!(lines.len(), 2, "CR LF ends one line");
    }

    #[test]
    fn a_carriage_return_rewrites_the_line() {
        let logs = ["10%\r50%\r100%\ndone".to_owned()];
        let lines = parse_logs(&logs, &DARK);
        let text: Vec<String> = lines
            .iter()
            .map(|line| line.iter().map(|(_, piece)| piece.as_str()).collect())
            .collect();
        assert_eq!(text, ["100%", "done"]);
    }
}
