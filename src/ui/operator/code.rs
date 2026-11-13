//! `CodeBlock` with the built-in shell highlighter (`docs/components/code-block.md`).

use eframe::egui::{
    self, Color32, Id, Rect, ScrollArea, Sense, Stroke, StrokeKind, TextFormat, Ui, pos2, text::LayoutJob,
    vec2,
};

use super::button::IconButton;
use super::icon;
use super::state::{Activity, BORDER, Outcome, result_hold};
use super::text::{galley, paint_aligned, paint_selectable, selectable_sense};
use super::tokens::{Palette, Type, dim, du, text};

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
        let (kind, len) = token(source, pos, c);
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

/// The kind and length of the token that starts at `pos` with `c`.
fn token(source: &str, pos: usize, c: char) -> (Option<Kind>, usize) {
    let rest = &source[pos..];
    let previous = source[..pos].chars().next_back();
    if c == '#' && previous.is_none_or(char::is_whitespace) {
        (Some(Kind::Comment), rest.find('\n').unwrap_or(rest.len()))
    } else if c == '"' || c == '\'' {
        (Some(Kind::String), quoted_len(rest, c, c == '"'))
    } else if c.is_ascii_digit() && !previous.is_some_and(is_word) {
        let len = rest
            .find(|d: char| !d.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        (Some(Kind::Number), len)
    } else if is_word_start(c) {
        let len = word_len(rest);
        (word_kind(&rest[..len], &rest[len..]), len)
    } else if c == '$' {
        (Some(Kind::Property), variable_len(rest))
    } else if PUNCTUATION.contains(c) {
        (Some(Kind::Punctuation), 1)
    } else {
        (None, c.len_utf8())
    }
}

/// A word is a keyword, a constant, a function when it is a builtin or a call, or plain.
fn word_kind(word: &str, after: &str) -> Option<Kind> {
    if KEYWORDS.contains(&word) {
        Some(Kind::Keyword)
    } else if CONSTANTS.contains(&word) {
        Some(Kind::Constant)
    } else if BUILTINS.contains(&word) || after.trim_start_matches([' ', '\t']).starts_with('(') {
        Some(Kind::Function)
    } else {
        None
    }
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

pub(super) fn format(colour: Color32) -> TextFormat {
    TextFormat {
        font_id: Type::Console.font(),
        line_height: Some(Type::Console.line()),
        color: colour,
        ..TextFormat::default()
    }
}

pub(super) fn job(wrap_width: Option<f32>) -> LayoutJob {
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
                    let at = area.min + vec2(pad, pad);
                    let text_rect = Rect::from_min_size(at, body.size());
                    let response = ui.interact(text_rect, self.id.with("text"), selectable_sense());
                    paint_selectable(ui, &response, at, body, palette.text_primary);
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
    fn numbers_constants_calls_and_punctuation_are_told_apart() {
        let code = "x1=42; ok=true; greet () { printf 'a\\b' 7z; } # note\nv2 # not a comment";
        assert_eq!(kind_of(code, "42"), Some(Kind::Number));
        assert_eq!(kind_of(code, "7z"), Some(Kind::Number));
        assert_eq!(kind_of(code, "x1"), None, "a digit inside a word is no number");
        assert_eq!(kind_of(code, "true"), Some(Kind::Constant));
        assert_eq!(
            kind_of(code, "greet"),
            Some(Kind::Function),
            "a word before ( is a call"
        );
        assert_eq!(kind_of(code, "printf"), Some(Kind::Function));
        assert_eq!(
            kind_of(code, "'a\\b'"),
            Some(Kind::String),
            "single quotes take no escapes"
        );
        assert_eq!(kind_of(code, "# note"), Some(Kind::Comment));
        assert_eq!(kind_of(code, "{"), Some(Kind::Punctuation));
        let tokens: String = shell_tokens(code).iter().map(|(_, text)| *text).collect();
        assert_eq!(tokens, code, "the tokens cover the code exactly");
    }

    #[test]
    fn every_kind_has_a_colour_of_its_own() {
        let kinds = [
            None,
            Some(Kind::Keyword),
            Some(Kind::String),
            Some(Kind::Number),
            Some(Kind::Comment),
            Some(Kind::Function),
            Some(Kind::Constant),
            Some(Kind::Property),
            Some(Kind::Punctuation),
        ];
        let mut colours: Vec<[u8; 4]> = kinds
            .iter()
            .map(|&kind| kind_colour(kind, &DARK).to_array())
            .collect();
        colours.sort_unstable();
        colours.dedup();
        assert_eq!(colours.len(), kinds.len());
    }
}
