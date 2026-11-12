//! `CodeBlock` with the built-in shell highlighter, and `ConsoleTerminal` with xterm SGR colours
//! (`docs/components/code-block.md`, `console-terminal.md`).

use eframe::egui::{
    self, Color32, Id, Rect, ScrollArea, Sense, Stroke, StrokeKind, TextFormat, Ui, pos2, text::LayoutJob,
    vec2,
};
use eframe::epaint::text::VariationCoords;

use super::button::IconButton;
use super::icon;
use super::state::{Activity, BORDER, Outcome, result_hold};
use super::text::{galley, paint_aligned};
use super::tokens::{BOLD, Palette, Type, dim, du, text};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Constant,
    Property,
    Punctuation,
}

const KEYWORDS: &[&str] = &[
    "case", "do", "done", "elif", "else", "esac", "fi", "for", "function", "if", "in", "select", "then",
    "time", "until", "while",
];
const CONSTANTS: &[&str] = &["true", "false"];
const BUILTINS: &[&str] = &[
    "alias", "cd", "echo", "eval", "exec", "exit", "export", "local", "printf", "read", "readonly", "return",
    "set", "shift", "source", "test", "trap", "unset", "wait",
];
const PUNCTUATION: &str = "{}[]()<>=+-*/%!&|^~?:;,.@#$\\";

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn word_len(s: &str) -> usize {
    s.char_indices()
        .find(|&(_, c)| !is_word(c))
        .map_or(s.len(), |(i, _)| i)
}

fn quoted_len(s: &str, quote: char, escapes: bool) -> usize {
    let mut chars = s.char_indices().skip(1);
    while let Some((i, c)) = chars.next() {
        if c == quote {
            return i + c.len_utf8();
        }
        if c == '\\' && escapes {
            chars.next();
        }
    }
    s.len()
}

fn variable_len(s: &str) -> usize {
    let rest = &s[1..];
    if rest.starts_with('{') {
        return rest.find('}').map_or(s.len(), |i| i + 2);
    }
    match rest.chars().next() {
        Some(c) if is_word_start(c) => 1 + word_len(rest),
        Some(c) if c.is_ascii_digit() || "@#?$!*-".contains(c) => 2,
        _ => 1,
    }
}

/// The shell rules of the design system scanner: comments, quotes, numbers, words, variables.
fn shell_tokens(source: &str) -> Vec<(Option<Kind>, &str)> {
    let mut out: Vec<(Option<Kind>, usize, usize)> = Vec::new();
    let mut pos = 0;
    while let Some(c) = source[pos..].chars().next() {
        let rest = &source[pos..];
        let previous = source[..pos].chars().next_back();
        let at_boundary = previous.is_none_or(char::is_whitespace);
        let (kind, len) = if c.is_whitespace() {
            (None, c.len_utf8())
        } else if c == '#' && at_boundary {
            (Some(Kind::Comment), rest.find('\n').unwrap_or(rest.len()))
        } else if c == '"' || c == '\'' {
            (Some(Kind::String), quoted_len(rest, c, c == '"'))
        } else if c.is_ascii_digit() && !previous.is_some_and(is_word) {
            (
                Some(Kind::Number),
                rest.find(|d: char| !d.is_ascii_alphanumeric())
                    .unwrap_or(rest.len()),
            )
        } else if is_word_start(c) {
            let len = word_len(rest);
            let word = &rest[..len];
            let kind = if KEYWORDS.contains(&word) {
                Some(Kind::Keyword)
            } else if CONSTANTS.contains(&word) {
                Some(Kind::Constant)
            } else if BUILTINS.contains(&word) || rest[len..].trim_start_matches([' ', '\t']).starts_with('(')
            {
                Some(Kind::Function)
            } else {
                None
            };
            (kind, len)
        } else if c == '$' {
            (Some(Kind::Property), variable_len(rest))
        } else if PUNCTUATION.contains(c) {
            (Some(Kind::Punctuation), 1)
        } else {
            (None, c.len_utf8())
        };
        let len = len.max(c.len_utf8());
        match out.last_mut() {
            Some((last, _, end)) if *last == kind && *end == pos => *end = pos + len,
            _ => out.push((kind, pos, pos + len)),
        }
        pos += len;
    }
    out.into_iter()
        .map(|(kind, start, end)| (kind, &source[start..end]))
        .collect()
}

fn kind_colour(kind: Option<Kind>, palette: &Palette) -> Color32 {
    match kind {
        None => palette.text_primary,
        Some(Kind::Keyword) => palette.syntax_keyword,
        Some(Kind::String) => palette.syntax_string,
        Some(Kind::Number) => palette.syntax_number,
        Some(Kind::Comment) => palette.syntax_comment,
        Some(Kind::Function) => palette.syntax_function,
        Some(Kind::Constant) => palette.syntax_constant,
        Some(Kind::Property) => palette.syntax_property,
        Some(Kind::Punctuation) => palette.syntax_punctuation,
    }
}

fn format(colour: Color32) -> TextFormat {
    TextFormat {
        font_id: Type::Console.font(),
        line_height: Some(Type::Console.line()),
        color: colour,
        ..TextFormat::default()
    }
}

fn job(wrap_width: Option<f32>) -> LayoutJob {
    let mut job = LayoutJob::default();
    match wrap_width {
        Some(width) => {
            job.wrap.max_width = width;
            job.wrap.break_anywhere = true;
        }
        None => job.wrap.max_width = f32::INFINITY,
    }
    job
}

pub struct CodeBlock<'a> {
    pub id: Id,
    pub code: &'a str,
    /// `sh` and its aliases are highlighted; other languages are shown plain.
    pub language: Option<&'a str>,
    pub wrap: bool,
    /// The accessible name of the copy button; `None` hides the button.
    pub copy: Option<&'a str>,
    pub max_height: Option<f32>,
}

impl CodeBlock<'_> {
    fn highlighted(&self, wrap_width: Option<f32>, palette: &Palette) -> LayoutJob {
        let mut job = job(wrap_width);
        let shell = self.language.is_some_and(|name| {
            ["bash", "sh", "shell", "zsh", "console"].contains(&name.to_ascii_lowercase().as_str())
        });
        if shell {
            for (kind, piece) in shell_tokens(self.code) {
                let mut look = format(kind_colour(kind, palette));
                look.italics = kind == Some(Kind::Comment);
                job.append(piece, 0.0, look);
            }
        } else {
            job.append(self.code, 0.0, format(palette.text_primary));
        }
        job
    }

    /// Draws the block. Returns whether its text was copied this frame.
    pub fn show(&self, ui: &mut Ui, palette: &Palette) -> bool {
        let width = ui.available_width();
        let pad = du(dim::PAD_CARD);
        let wrap_width = self.wrap.then_some(width - 2.0 * pad);
        let body = ui.fonts_mut(|fonts| fonts.layout_job(self.highlighted(wrap_width, palette)));
        let has_head = self.language.is_some() || self.copy.is_some();
        let head = if has_head { du(dim::ROW_COMPACT) } else { 0.0 };
        let content = body.size().y + 2.0 * pad;
        let visible = self.max_height.map_or(content, |limit| content.min(du(limit)));
        let (rect, _) = ui.allocate_exact_size(vec2(width, head + visible), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, palette.surface_alt);
        let mut copied = false;
        if has_head {
            let bar = Rect::from_min_size(rect.min, vec2(width, head));
            copied = self.head(ui, bar, palette);
        }
        let body_rect = Rect::from_min_max(pos2(rect.left(), rect.top() + head), rect.max);
        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            let scroll = if self.wrap {
                ScrollArea::vertical()
            } else {
                ScrollArea::both()
            };
            scroll
                .id_salt(self.id.with("body"))
                .max_height(visible)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let size = body.size() + vec2(2.0 * pad, 2.0 * pad);
                    let (area, _) =
                        ui.allocate_exact_size(size.max(vec2(body_rect.width(), 0.0)), Sense::hover());
                    ui.painter()
                        .galley(area.min + vec2(pad, pad), body, palette.text_primary);
                });
        });
        ui.painter()
            .rect_stroke(rect, 0.0, Stroke::new(BORDER, palette.border), StrokeKind::Inside);
        copied
    }

    fn head(&self, ui: &mut Ui, bar: Rect, palette: &Palette) -> bool {
        let label_rect = bar.shrink2(vec2(du(dim::PAD_CARD), 0.0));
        if let Some(language) = self.language {
            let label = galley(
                ui,
                text(language.to_uppercase(), Type::Caption, palette.text_secondary),
            );
            paint_aligned(ui, label_rect, label, egui::Align::Min, palette.text_secondary);
        }
        let mut copied = false;
        if let Some(name) = self.copy {
            let key = self.id.with("copy");
            let mut activity = Activity::load(ui.ctx(), key);
            let now = ui.input(|input| input.time);
            let side = du(dim::ROW_DENSE);
            let button = Rect::from_min_size(
                pos2(
                    bar.right() - du(dim::PAD_TIGHT) - side,
                    bar.center().y - side / 2.0,
                ),
                vec2(side, side),
            );
            let look = activity.look(now, result_hold());
            let clicked = ui
                .scope_builder(egui::UiBuilder::new().max_rect(button), |ui| {
                    IconButton::new(icon::CONTENT_COPY, name)
                        .size(dim::ROW_DENSE)
                        .icon_size(3.5)
                        .look(look)
                        .show(ui, palette)
                        .clicked()
                })
                .inner;
            if clicked {
                ui.ctx().copy_text(self.code.to_owned());
                activity.start(now);
                activity.finish(Outcome::Success, now);
                copied = true;
            }
            activity.store(ui.ctx(), key);
        }
        ui.painter().hline(
            bar.x_range(),
            bar.bottom() - BORDER / 2.0,
            Stroke::new(BORDER, palette.divider),
        );
        copied
    }
}

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

fn apply_sgr(style: &mut Style, params: &[u16], palette: &Palette) {
    if params.is_empty() {
        *style = Style::default();
        return;
    }
    let colours = palette.terminal();
    let mut i = 0;
    while i < params.len() {
        let code = params[i];
        i += 1;
        match code {
            0 => *style = Style::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.inverse = true,
            9 => style.strike = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.inverse = false,
            29 => style.strike = false,
            30..=37 => style.foreground = Some(colours[usize::from(code - 30)]),
            40..=47 => style.background = Some(colours[usize::from(code - 40)]),
            90..=97 => style.foreground = Some(colours[usize::from(code - 90 + 8)]),
            100..=107 => style.background = Some(colours[usize::from(code - 100 + 8)]),
            38 | 48 => {
                let (colour, used) = extended(&params[i..], palette);
                i += used;
                if code == 38 {
                    style.foreground = colour;
                } else {
                    style.background = colour;
                }
            }
            39 => style.foreground = None,
            49 => style.background = None,
            _ => {}
        }
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
                '\u{1b}' => match chars.next() {
                    Some('[') => {
                        let mut body = String::new();
                        let mut last = None;
                        for next in chars.by_ref() {
                            if ('@'..='~').contains(&next) {
                                last = Some(next);
                                break;
                            }
                            body.push(next);
                        }
                        if last == Some('m') {
                            let params: Vec<u16> = body
                                .split([';', ':'])
                                .filter(|part| !part.is_empty())
                                .filter_map(|part| part.parse().ok())
                                .collect();
                            apply_sgr(&mut style, &params, palette);
                        }
                    }
                    Some(']') => {
                        while let Some(next) = chars.next() {
                            if next == '\u{7}' || (next == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                                break;
                            }
                        }
                    }
                    Some('(' | ')') => {
                        chars.next();
                    }
                    _ => {}
                },
                '\n' => lines.push(std::mem::take(&mut line)),
                '\r' if chars.peek() == Some(&'\n') => {}
                '\r' => line.clear(),
                _ => match line.last_mut() {
                    Some((last, piece)) if *last == style => piece.push(c),
                    _ => line.push((style, c.to_string())),
                },
            }
        }
        lines.push(line);
    }
    lines
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

    fn kind_of(code: &str, piece: &str) -> Option<Kind> {
        shell_tokens(code)
            .into_iter()
            .find(|(_, text)| *text == piece)
            .and_then(|(kind, _)| kind)
    }

    #[test]
    fn shell_tokens_follow_the_design_system() {
        let code = "# deploy\nexport PORT=8080\nif [ \"$1\" = up ]; then echo ${HOME} $#; fi";
        assert_eq!(kind_of(code, "# deploy"), Some(Kind::Comment));
        assert_eq!(kind_of(code, "export"), Some(Kind::Function));
        assert_eq!(kind_of(code, "if"), Some(Kind::Keyword));
        assert_eq!(kind_of(code, "\"$1\""), Some(Kind::String));
        assert_eq!(kind_of(code, "${HOME}"), Some(Kind::Property));
        assert_eq!(kind_of(code, "$#"), Some(Kind::Property));
    }

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
