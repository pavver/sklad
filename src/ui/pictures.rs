//! Icons of programs, decoded on one thread and only for the rows and cards actually shown.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

use super::operator::content::Picture;
use crate::icons::{Decoded, decode};

enum Slot {
    Waiting,
    Ready(TextureHandle),
    Missing,
}

struct Answer {
    path: PathBuf,
    image: Option<Decoded>,
}

pub struct Pictures {
    slots: HashMap<PathBuf, Slot>,
    requests: Sender<PathBuf>,
    answers: Receiver<Answer>,
}

impl Pictures {
    pub fn new(ctx: &egui::Context) -> Self {
        let (requests, incoming) = channel::<PathBuf>();
        let (reply, answers) = channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            for path in incoming {
                let image = decode(&path);
                if reply.send(Answer { path, image }).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
        Self {
            slots: HashMap::new(),
            requests,
            answers,
        }
    }

    /// Takes what the decoder finished. Called once a frame.
    pub fn receive(&mut self, ctx: &egui::Context) {
        while let Ok(answer) = self.answers.try_recv() {
            let slot = answer.image.map_or(Slot::Missing, |image| {
                let picture = ColorImage::from_rgba_premultiplied(image.size, &image.rgba);
                let name = answer.path.display().to_string();
                Slot::Ready(ctx.load_texture(name, picture, TextureOptions::LINEAR))
            });
            self.slots.insert(answer.path, slot);
        }
    }

    /// The picture of an icon path, asking for it the first time.
    pub fn get(&mut self, path: Option<&Path>) -> Picture<'_> {
        let Some(path) = path else {
            return Picture::Missing;
        };
        if !self.slots.contains_key(path) {
            let asked = self.requests.send(path.to_path_buf()).is_ok();
            let slot = if asked { Slot::Waiting } else { Slot::Missing };
            self.slots.insert(path.to_path_buf(), slot);
        }
        match self.slots.get(path) {
            Some(Slot::Ready(texture)) => Picture::Ready(texture),
            Some(Slot::Waiting) => Picture::Loading,
            _ => Picture::Missing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Takes answers until `path` is no longer waiting, for at most a few seconds.
    fn wait(pictures: &mut Pictures, ctx: &egui::Context, path: &Path) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            pictures.receive(ctx);
            if !matches!(pictures.get(Some(path)), Picture::Loading) {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn a_picture_is_asked_for_once_and_comes_ready_or_missing() -> std::io::Result<()> {
        let ctx = egui::Context::default();
        let mut pictures = Pictures::new(&ctx);
        assert!(matches!(pictures.get(None), Picture::Missing));

        let folder = std::env::temp_dir().join(format!("sklad-pictures-{}", std::process::id()));
        std::fs::create_dir_all(&folder)?;
        let icon = folder.join("icon.svg");
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8"/></svg>"#;
        std::fs::write(&icon, svg)?;
        let broken = folder.join("broken.png");
        std::fs::write(&broken, "not a picture")?;

        assert!(matches!(pictures.get(Some(&icon)), Picture::Loading));
        let icon_done = wait(&mut pictures, &ctx, &icon);
        let broken_done = wait(&mut pictures, &ctx, &broken);
        let _ = std::fs::remove_dir_all(&folder);
        assert!(icon_done && broken_done, "the decoder answers");
        assert!(matches!(pictures.get(Some(&icon)), Picture::Ready(_)));
        assert!(matches!(pictures.get(Some(&broken)), Picture::Missing));
        Ok(())
    }
}
