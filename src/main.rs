#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cabinet;
mod domain;
mod emulators;
mod ui;

/// Default is the fullscreen cabinet UI; `--desktop` opens the windowed
/// egui interface for setup chores (ROM directory, wheel, resolution).
fn main() -> anyhow::Result<()> {
    env_logger::init();
    if std::env::args().any(|arg| arg == "--desktop") {
        desktop()
    } else {
        cabinet::run()
    }
}

fn desktop() -> anyhow::Result<()> {
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
    .map_err(|e| anyhow::anyhow!("desktop UI failed: {e}"))
}
