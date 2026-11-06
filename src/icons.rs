//! Finds icons by name in the installed themes and decodes them off the interface thread.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::scan::Places;

/// Pixel size icons are decoded at: sharp at the largest size the interface draws.
pub const DECODED_SIZE: u32 = 96;

#[derive(Clone, Debug, Default)]
pub struct IconIndex {
    found: HashMap<String, Candidate>,
}

#[derive(Clone, Debug)]
struct Candidate {
    path: PathBuf,
    rank: u32,
}

/// A vector beats any bitmap; among bitmaps, the larger, because a small one blurs when scaled up.
fn rank(path: &Path, size_hint: Option<u32>, theme_order: u32) -> u32 {
    let quality = match path.extension().and_then(|e| e.to_str()) {
        Some("svg") => 10_000,
        _ => size_hint.unwrap_or(16).min(9_999),
    };
    (100 - theme_order.min(99)) * 100_000 + quality
}

fn size_in(name: &str) -> Option<u32> {
    name.split('x').next()?.parse().ok()
}

fn is_supported(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("svg" | "png" | "jpg" | "jpeg")
    )
}

impl IconIndex {
    fn offer(&mut self, path: PathBuf, size_hint: Option<u32>, theme_order: u32) {
        if !is_supported(&path) {
            return;
        }
        let Some(name) = path.file_stem().map(|n| n.to_string_lossy().into_owned()) else {
            return;
        };
        let rank = rank(&path, size_hint, theme_order);
        let slot = self.found.entry(name).or_insert(Candidate {
            path: path.clone(),
            rank: 0,
        });
        if rank > slot.rank {
            *slot = Candidate { path, rank };
        }
    }

    fn add_folder(&mut self, folder: &Path, size_hint: Option<u32>, theme_order: u32) {
        let Ok(entries) = fs::read_dir(folder) else {
            return;
        };
        for entry in entries.flatten() {
            self.offer(entry.path(), size_hint, theme_order);
        }
    }

    /// Themes lay folders out as `48x48/apps` or `apps/48`; both are looked at.
    fn add_theme(&mut self, theme: &Path, theme_order: u32) {
        let Ok(entries) = fs::read_dir(theme) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "apps" {
                let Ok(sizes) = fs::read_dir(&path) else {
                    continue;
                };
                for size in sizes.flatten() {
                    let hint = size_in(&size.file_name().to_string_lossy());
                    self.add_folder(&size.path(), hint, theme_order);
                }
            } else {
                self.add_folder(&path.join("apps"), size_in(&name), theme_order);
            }
        }
    }

    pub fn build(places: &Places) -> Self {
        let home = &places.home;
        let kdeglobals = fs::read_to_string(home.join(".config/kdeglobals")).unwrap_or_default();
        let current = current_theme(&kdeglobals);
        let bases = [
            home.join(".local/share/icons"),
            home.join(".icons"),
            places.root.join("usr/share/icons"),
            places.root.join("var/lib/flatpak/exports/share/icons"),
            home.join(".local/share/flatpak/exports/share/icons"),
        ];
        let mut themes: Vec<String> = current.into_iter().collect();
        if let Some(parent) = themes
            .first()
            .and_then(|t| t.strip_suffix("-Dark"))
            .map(str::to_owned)
        {
            themes.push(parent);
        }
        themes.extend(["breeze-dark", "breeze", "hicolor"].map(str::to_owned));

        let mut index = Self::default();
        for (order, theme) in themes.iter().enumerate() {
            for base in &bases {
                index.add_theme(&base.join(theme), order as u32);
            }
        }
        index.add_folder(
            &places.root.join("usr/share/pixmaps"),
            Some(48),
            themes.len() as u32,
        );
        index
    }

    /// An icon by the name a launcher gives, which may also be a path.
    pub fn find(&self, name: &str) -> Option<PathBuf> {
        let path = Path::new(name);
        if path.is_absolute() {
            return (is_supported(path) && path.is_file()).then(|| path.to_path_buf());
        }
        self.found.get(name).map(|candidate| candidate.path.clone())
    }
}

fn current_theme(kdeglobals: &str) -> Option<String> {
    let mut in_icons = false;
    for line in kdeglobals.lines() {
        if line.starts_with('[') {
            in_icons = line == "[Icons]";
        } else if let Some(theme) = line.strip_prefix("Theme=").filter(|_| in_icons) {
            return Some(theme.trim().to_owned());
        }
    }
    None
}

/// A decoded image, RGBA with premultiplied alpha, `size` pixels on a side.
#[derive(Clone, Debug)]
pub struct Decoded {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

pub fn decode(path: &Path) -> Option<Decoded> {
    let bytes = fs::read(path).ok()?;
    if path.extension().is_some_and(|extension| extension == "svg") {
        decode_svg(&bytes)
    } else {
        decode_bitmap(&bytes)
    }
}

fn decode_svg(bytes: &[u8]) -> Option<Decoded> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(DECODED_SIZE, DECODED_SIZE)?;
    let source = tree.size();
    let scale = DECODED_SIZE as f32 / source.width().max(source.height());
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(Decoded {
        size: [DECODED_SIZE as usize; 2],
        rgba: pixmap.take(),
    })
}

fn decode_bitmap(bytes: &[u8]) -> Option<Decoded> {
    let image = image::load_from_memory(bytes).ok()?;
    let image = image.thumbnail(DECODED_SIZE, DECODED_SIZE).into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let mut rgba = image.into_raw();
    for pixel in rgba.as_chunks_mut::<4>().0 {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = (u16::from(*channel) * alpha / 255) as u8;
        }
    }
    Some(Decoded { size, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vector_beats_a_bitmap_and_the_chosen_theme_beats_all() {
        let vector = rank(Path::new("a.svg"), None, 2);
        let bitmap = rank(Path::new("a.png"), Some(256), 2);
        let themed = rank(Path::new("a.png"), Some(16), 0);
        assert!(vector > bitmap && themed > vector);
    }

    #[test]
    fn the_theme_is_read_from_its_group() {
        let text = "[General]\nTheme=wrong\n[Icons]\nTheme=Papirus-Dark\n";
        assert_eq!(current_theme(text).as_deref(), Some("Papirus-Dark"));
    }

    #[test]
    fn an_svg_is_rasterised_at_full_size() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
            <rect width="16" height="16" fill="red"/></svg>"#;
        let image = decode_svg(svg);
        assert_eq!(image.as_ref().map(|i| i.size), Some([96, 96]));
        assert_eq!(image.map(|i| i.rgba[0]), Some(255));
    }

    #[test]
    fn a_file_that_is_no_image_gives_nothing() {
        assert!(decode_bitmap(b"not an image").is_none());
    }
}
