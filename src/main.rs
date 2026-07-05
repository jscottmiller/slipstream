#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod domain;
mod emulators;
mod ui;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Slipstream")
            .with_inner_size([920.0, 600.0])
            .with_min_inner_size([720.0, 460.0]),
        ..Default::default()
    };

    eframe::run_native(
        "slipstream",
        options,
        Box::new(|cc| Ok(Box::new(app::SlipstreamApp::new(cc)))),
    )
}
