//! DJ Library Manager: pantalla principal (egui/eframe 0.29).

mod app;
mod artwork;
mod covers;
mod detection;
mod harmony;
mod i18n;
mod model;
mod process_guard;
mod readers;
#[cfg(test)]
mod sample;
mod theme;
mod ui;

use std::sync::Arc;

use app::App;
use detection::LocalProvider;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 740.0])
            .with_min_inner_size([820.0, 520.0])
            .with_title(i18n::ES.app_title),
        ..Default::default()
    };

    // Proveedor real: Rekordbox (exportación XML) y Traktor (`collection.nml`),
    // siempre en solo lectura y con el programa cerrado.
    let provider = Arc::new(LocalProvider::new());

    eframe::run_native(
        i18n::ES.app_title,
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, provider)))),
    )
}
