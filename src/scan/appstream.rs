//! AppStream metadata: the long description, the licence and the developer, as Discover shows them.

use std::fs;
use std::path::Path;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metainfo {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub developer: Option<String>,
}

pub fn read(path: &Path, language: &str) -> Option<Metainfo> {
    parse(&fs::read_to_string(path).ok()?, language)
}

/// One paragraph or list item in the plain language or in the reader's one.
#[derive(Default)]
struct Text {
    plain: Vec<String>,
    local: Vec<String>,
}

impl Text {
    fn push(&mut self, local: bool, text: String) {
        if local {
            self.local.push(text);
        } else {
            self.plain.push(text);
        }
    }

    fn joined(self) -> Option<String> {
        let parts = if self.local.is_empty() {
            self.plain
        } else {
            self.local
        };
        (!parts.is_empty()).then(|| parts.join("\n\n"))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    Plain,
    Local,
    Foreign,
}

fn language_of(tag: &BytesStart<'_>, language: &str) -> Lang {
    let attribute = tag
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == b"xml:lang");
    match attribute {
        None => Lang::Plain,
        Some(attribute) if attribute.value.as_ref() == language.as_bytes() => Lang::Local,
        Some(_) => Lang::Foreign,
    }
}

fn attribute_is(tag: &BytesStart<'_>, key: &[u8], value: &[u8]) -> bool {
    tag.attributes()
        .flatten()
        .any(|attribute| attribute.key.as_ref() == key && attribute.value.as_ref() == value)
}

/// The element whose text is being collected.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Summary,
    Paragraph,
    ListEntry,
    License,
    Homepage,
    Developer,
}

pub fn parse(xml: &str, language: &str) -> Option<Metainfo> {
    let mut reader = Reader::from_str(xml);
    let mut info = Metainfo::default();
    let mut description = Text::default();
    let mut summary = Text::default();
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut open: Option<(Field, Lang)> = None;
    let mut buffer = String::new();

    loop {
        match reader.read_event().ok()? {
            Event::Start(tag) => {
                let name = tag.name().as_ref().to_vec();
                let depth = path.len();
                let parent = path.last().map(Vec::as_slice);
                let lang = language_of(&tag, language);
                let field = match name.as_slice() {
                    b"summary" if depth == 1 => Some(Field::Summary),
                    b"p" if path.iter().any(|p| p == b"description") && depth <= 3 => Some(Field::Paragraph),
                    b"li" if path.iter().any(|p| p == b"description") => Some(Field::ListEntry),
                    b"project_license" if depth == 1 => Some(Field::License),
                    b"url" if depth == 1 && attribute_is(&tag, b"type", b"homepage") => Some(Field::Homepage),
                    b"name" if parent == Some(b"developer") => Some(Field::Developer),
                    b"developer_name" if depth == 1 => Some(Field::Developer),
                    _ => None,
                };
                if let Some(field) = field.filter(|_| open.is_none()) {
                    open = Some((field, lang));
                    buffer.clear();
                }
                path.push(name);
            }
            Event::Text(text) => {
                if open.is_some() {
                    buffer.push_str(&text.decode().ok()?);
                }
            }
            Event::GeneralRef(entity) => {
                if open.is_some() {
                    let name = entity.decode().ok()?;
                    let resolved = quick_xml::escape::resolve_xml_entity(&name);
                    buffer.push_str(resolved.unwrap_or_default());
                }
            }
            Event::End(tag) => {
                let name = tag.name().as_ref().to_vec();
                path.pop();
                let closes = matches!(
                    (open.map(|(field, _)| field), name.as_slice()),
                    (Some(Field::Summary), b"summary")
                        | (Some(Field::Paragraph), b"p")
                        | (Some(Field::ListEntry), b"li")
                        | (Some(Field::License), b"project_license")
                        | (Some(Field::Homepage), b"url")
                        | (Some(Field::Developer), b"name" | b"developer_name")
                );
                if !closes {
                    continue;
                }
                let Some((field, lang)) = open.take() else {
                    continue;
                };
                let text = buffer.split_whitespace().collect::<Vec<_>>().join(" ");
                if lang == Lang::Foreign || text.is_empty() {
                    continue;
                }
                let local = lang == Lang::Local;
                match field {
                    Field::Summary => summary.push(local, text),
                    Field::Paragraph => description.push(local, text),
                    Field::ListEntry => description.push(local, format!("- {text}")),
                    Field::License => info.license = Some(text),
                    Field::Homepage => info.homepage = Some(text),
                    Field::Developer => {
                        info.developer.get_or_insert(text);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    info.description = description.joined();
    info.summary = summary.joined();
    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<component type="desktop">
  <id>app</id>
  <project_license>GPL-2.0-or-later</project_license>
  <developer id="org.kde"><name translate="no">KDE</name></developer>
  <summary>File Manager</summary>
  <summary xml:lang="de">Dateien</summary>
  <summary xml:lang="uk">Керування файлами</summary>
  <description>
    <p>Plain &amp; simple.</p>
    <p xml:lang="de">Einfach.</p>
    <p xml:lang="uk">Просто.</p>
    <ul><li xml:lang="uk">Швидко</li><li>Fast</li></ul>
  </description>
  <url type="bugtracker">https://bugs.example</url>
  <url type="homepage">https://example.org</url>
</component>"#;

    #[test]
    fn an_english_reader_gets_the_plain_text() {
        let info = parse(SAMPLE, "en").unwrap_or_default();
        assert_ne!(info.summary.as_deref(), Some("Керування файлами"));
        assert!(info.summary.is_some());
    }

    #[test]
    fn the_translation_is_taken_and_other_languages_dropped() {
        let info = parse(SAMPLE, "uk").unwrap_or_default();
        assert_eq!(info.summary.as_deref(), Some("Керування файлами"));
        assert_eq!(info.description.as_deref(), Some("Просто.\n\n- Швидко"));
    }

    #[test]
    fn the_facts_are_read() {
        let info = parse(SAMPLE, "uk").unwrap_or_default();
        assert_eq!(info.license.as_deref(), Some("GPL-2.0-or-later"));
        assert_eq!(info.developer.as_deref(), Some("KDE"));
        assert_eq!(info.homepage.as_deref(), Some("https://example.org"));
    }

    #[test]
    fn without_a_translation_the_plain_text_stays_and_entities_survive() {
        let xml = "<component><description><p>Plain &amp; simple</p></description></component>";
        let info = parse(xml, "uk").unwrap_or_default();
        assert_eq!(info.description.as_deref(), Some("Plain & simple"));
    }
}
