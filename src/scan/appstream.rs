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

impl Field {
    /// The field an element opens, from its name and where it sits: `path` holds the elements
    /// around it, the root first.
    fn of(tag: &BytesStart<'_>, name: &[u8], path: &[Vec<u8>]) -> Option<Field> {
        let depth = path.len();
        let in_description = || path.iter().any(|outer| outer == b"description");
        match name {
            b"summary" if depth == 1 => Some(Field::Summary),
            b"p" if in_description() && depth <= 3 => Some(Field::Paragraph),
            b"li" if in_description() => Some(Field::ListEntry),
            b"project_license" if depth == 1 => Some(Field::License),
            b"url" if depth == 1 && attribute_is(tag, b"type", b"homepage") => Some(Field::Homepage),
            b"name" if path.last().is_some_and(|parent| parent == b"developer") => Some(Field::Developer),
            b"developer_name" if depth == 1 => Some(Field::Developer),
            _ => None,
        }
    }
}

/// The field being collected: what it is, its language, and the element that closes it.
struct Open {
    field: Field,
    lang: Lang,
    tag: Vec<u8>,
}

/// What the reading has gathered so far.
#[derive(Default)]
struct Gathered {
    info: Metainfo,
    description: Text,
    summary: Text,
}

impl Gathered {
    /// Keeps the text of a closed field, unless it is empty or in another language.
    fn keep(&mut self, open: Open, text: String) {
        if open.lang == Lang::Foreign || text.is_empty() {
            return;
        }
        let local = open.lang == Lang::Local;
        match open.field {
            Field::Summary => self.summary.push(local, text),
            Field::Paragraph => self.description.push(local, text),
            Field::ListEntry => self.description.push(local, format!("- {text}")),
            Field::License => self.info.license = Some(text),
            Field::Homepage => self.info.homepage = Some(text),
            Field::Developer => {
                self.info.developer.get_or_insert(text);
            }
        }
    }

    fn finish(self) -> Metainfo {
        Metainfo {
            description: self.description.joined(),
            summary: self.summary.joined(),
            ..self.info
        }
    }
}

pub fn parse(xml: &str, language: &str) -> Option<Metainfo> {
    let mut reader = Reader::from_str(xml);
    let mut gathered = Gathered::default();
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut open: Option<Open> = None;
    let mut buffer = String::new();
    loop {
        match reader.read_event().ok()? {
            Event::Start(tag) => {
                let name = tag.name().as_ref().to_vec();
                if open.is_none()
                    && let Some(field) = Field::of(&tag, &name, &path)
                {
                    let lang = language_of(&tag, language);
                    open = Some(Open {
                        field,
                        lang,
                        tag: name.clone(),
                    });
                    buffer.clear();
                }
                path.push(name);
            }
            Event::Text(text) if open.is_some() => buffer.push_str(&text.decode().ok()?),
            Event::GeneralRef(entity) if open.is_some() => {
                let name = entity.decode().ok()?;
                buffer.push_str(quick_xml::escape::resolve_xml_entity(&name).unwrap_or_default());
            }
            Event::End(tag) => {
                path.pop();
                if let Some(closed) = open.take_if(|open| open.tag == tag.name().as_ref()) {
                    gathered.keep(closed, buffer.split_whitespace().collect::<Vec<_>>().join(" "));
                }
            }
            Event::Eof => return Some(gathered.finish()),
            _ => {}
        }
    }
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
