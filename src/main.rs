use std::sync::Arc;

use eframe::egui;
use sklad::i18n::Choice;
use sklad::remove::SystemBackend;
use sklad::scan::{self, Places};
use sklad::ui::App;

fn main() -> eframe::Result {
    let places = Places::system();
    let choice = Choice::load(Choice::default_file(&places.home));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(choice.lang.text().app_name)
            .with_app_id("sklad")
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([1100.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "sklad",
        options,
        Box::new(|creation| {
            let scanner = Arc::new(move |lang| scan::run(&places, lang));
            Ok(Box::new(App::new(
                &creation.egui_ctx,
                Arc::new(SystemBackend),
                scanner,
                choice,
            )))
        }),
    )
}
