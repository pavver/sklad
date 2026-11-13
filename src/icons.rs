//! Finds icons by name in the installed themes and decodes them off the interface thread.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::scan::Places;

/// Pixel size icons are decoded at: sharp at the largest size the interface draws.
pub const DECODED_SIZE: u32 = 96;

/// The folders of a theme the index reads.
const CONTEXTS: [&str; 5] = ["apps", "categories", "places", "devices", "mimetypes"];

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

    /// Themes lay folders out as `48x48/apps` or `apps/48`; both are looked at. Besides the
    /// icons of programs, the side bar takes those of categories, places, devices and file
    /// types.
    fn add_theme(&mut self, theme: &Path, theme_order: u32) {
        for entry in fs::read_dir(theme).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if CONTEXTS.contains(&name.as_str()) {
                self.add_sizes(&entry.path(), theme_order);
            } else {
                for context in CONTEXTS {
                    self.add_folder(&entry.path().join(context), size_in(&name), theme_order);
                }
            }
        }
    }

    /// The size folders inside a context folder, as in `apps/48`.
    fn add_sizes(&mut self, context: &Path, theme_order: u32) {
        for size in fs::read_dir(context).into_iter().flatten().flatten() {
            let hint = size_in(&size.file_name().to_string_lossy());
            self.add_folder(&size.path(), hint, theme_order);
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

/// The icon of the program for the window. It is opaque, so the premultiplied pixels of the renderer
/// are the plain ones the window system expects.
pub fn window_icon() -> Option<eframe::egui::IconData> {
    let decoded = decode_svg(include_bytes!("../assets/sklad.svg"))?;
    Some(eframe::egui::IconData {
        rgba: decoded.rgba,
        width: decoded.size[0] as u32,
        height: decoded.size[1] as u32,
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
    fn the_window_icon_is_a_square_opaque_picture() {
        let icon = window_icon().unwrap_or_default();
        assert!(icon.width > 0);
        assert_eq!(icon.width, icon.height);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        let pixels = icon.rgba.as_chunks::<4>().0;
        assert!(pixels.iter().all(|pixel| pixel[3] == 255));
        assert!(pixels.iter().any(|pixel| *pixel != pixels[0]));
    }

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

    fn touch(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        fs::write(path, bytes)
    }

    #[test]
    fn icons_are_found_in_the_chosen_theme_its_parent_and_the_fallbacks() -> std::io::Result<()> {
        let base = std::env::temp_dir().join(format!("sklad-icons-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let icons = base.join("usr/share/icons");
        touch(
            &base.join("home/.config/kdeglobals"),
            b"[Icons]\nTheme=Papirus-Dark\n",
        )?;
        touch(&icons.join("Papirus-Dark/48x48/apps/gimp.png"), b"")?;
        touch(&icons.join("hicolor/scalable/apps/gimp.svg"), b"")?;
        touch(&icons.join("Papirus/apps/64/inkscape.png"), b"")?;
        touch(
            &icons.join("hicolor/48x48/categories/applications-games.png"),
            b"",
        )?;
        touch(&icons.join("hicolor/48x48/actions/edit-copy.png"), b"")?;
        touch(&icons.join("hicolor/48x48/apps/notes.txt"), b"")?;
        touch(&base.join("usr/share/pixmaps/old.png"), b"")?;
        let places = Places {
            home: base.join("home"),
            pacman_local: base.join("pacman"),
            dpkg_status: base.join("status"),
            apt_extended_states: base.join("states"),
            os_release: base.join("os-release"),
            root: base.clone(),
        };
        let index = IconIndex::build(&places);
        let found = |name: &str| {
            index
                .find(name)
                .and_then(|path| path.strip_prefix(&base).ok().map(Path::to_path_buf))
        };
        let result = [
            found("gimp"),
            found("inkscape"),
            found("applications-games"),
            found("edit-copy"),
            found("notes"),
            found("old"),
        ];
        let absolute = index.find(&icons.join("Papirus/apps/64/inkscape.png").to_string_lossy());
        let _ = fs::remove_dir_all(&base);
        let path = |text: &str| Some(PathBuf::from(text));
        assert_eq!(
            result[0],
            path("usr/share/icons/Papirus-Dark/48x48/apps/gimp.png"),
            "the chosen theme first"
        );
        assert_eq!(
            result[1],
            path("usr/share/icons/Papirus/apps/64/inkscape.png"),
            "then its light parent"
        );
        assert_eq!(
            result[2],
            path("usr/share/icons/hicolor/48x48/categories/applications-games.png")
        );
        assert_eq!(result[3], None, "actions are not read");
        assert_eq!(result[4], None, "only pictures count");
        assert_eq!(result[5], path("usr/share/pixmaps/old.png"));
        assert!(absolute.is_some(), "a launcher may give a path");
        Ok(())
    }

    #[test]
    fn a_bitmap_is_decoded_with_premultiplied_alpha() -> Result<(), image::ImageError> {
        let picture = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 128]));
        let mut png = std::io::Cursor::new(Vec::new());
        picture.write_to(&mut png, image::ImageFormat::Png)?;
        let decoded = decode_bitmap(png.get_ref());
        assert_eq!(
            decoded.as_ref().map(|image| image.size),
            Some([96, 96]),
            "scaled to the decoded size"
        );
        assert_eq!(
            decoded.map(|image| image.rgba[..4].to_vec()),
            Some(vec![128, 0, 0, 128])
        );
        Ok(())
    }

    #[test]
    fn a_file_that_is_no_image_gives_nothing() {
        assert!(decode(Path::new("/nonexistent/icon.svg")).is_none());
        assert!(decode_bitmap(b"not an image").is_none());
    }
}
