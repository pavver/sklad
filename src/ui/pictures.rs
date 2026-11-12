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
