#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cabinet;
mod domain;
mod emulators;
mod ui;

/// With no arguments, Settings → Interface decides which UI opens
/// (cabinet fullscreen by default); `--cabinet` / `--desktop` force one.
/// Alt+Enter in either UI switches to the other.
fn main() -> anyhow::Result<()> {
    env_logger::init();
    let arg = |flag: &str| std::env::args().any(|a| a == flag);
    if arg("--desktop") {
        desktop()
    } else if arg("--cabinet") {
        cabinet::run()
    } else {
        let paths = domain::paths::AppPaths::resolve()?;
        match domain::settings::Settings::load(&paths).default_ui {
            domain::settings::DefaultUi::Desktop => desktop(),
            domain::settings::DefaultUi::Cabinet => cabinet::run(),
        }
    }
}

/// Hand off to the other interface as a fresh process; the caller then
/// exits its own event loop. A process boundary sidesteps re-initializing
/// windowing stacks in-process (winit event loops don't like being
/// recreated, SDL teardown has platform quirks), and settings already
/// travel through config.toml. The target is always explicit — the
/// counterpart of a UI is the other UI, whatever default_ui says.
pub(crate) fn spawn_counterpart(desktop: bool) -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe)
        .arg(if desktop { "--desktop" } else { "--cabinet" })
        .spawn()
        .map(drop)
        .map_err(Into::into)
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
