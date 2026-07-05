use crate::app::{Action, SlipstreamApp};
use crate::domain::game::GAMES;
use crate::domain::{emulator, status, wheel};
use eframe::egui;

pub(crate) fn show(app: &mut SlipstreamApp, ui: &mut egui::Ui) -> Option<Action> {
    let mut action = None;

    egui::Panel::left("game_list")
        .default_size(280.0)
        .show(ui, |ui| {
            ui.add_space(6.0);
            for (i, game) in GAMES.iter().enumerate() {
                let st = status::game_status(game, &app.settings, &app.paths);
                let marker = if st.ready() { "▶" } else { "…" };
                let label = format!("{marker}  {}", game.title);
                if ui.selectable_label(app.selected == i, label).clicked() {
                    app.selected = i;
                }
            }
        });

    egui::CentralPanel::default().show(ui, |ui| {
        let game = &GAMES[app.selected.min(GAMES.len() - 1)];
        let st = status::game_status(game, &app.settings, &app.paths);
        let emu = emulator::find(game.emulator_id);
        let installing = app.installs.get(game.emulator_id);

        ui.add_space(8.0);
        ui.heading(game.title);
        ui.label(format!(
            "{} · {} · {}",
            game.system.label(),
            game.year,
            game.manufacturer
        ));
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Emulator status
        match emu {
            Some(emu) => {
                if st.emulator_installed {
                    ui.label(format!("✔ Emulator: {} (installed)", emu.name()));
                } else {
                    ui.label(format!("✘ Emulator: {} — not installed", emu.name()));
                }
            }
            None => {
                ui.colored_label(
                    egui::Color32::LIGHT_RED,
                    format!("No emulator registered for {}", game.emulator_id),
                );
            }
        }

        // ROM status
        match (&app.settings.rom_dir, st.rom_found, &st.rom_path) {
            (None, ..) => {
                ui.label("✘ ROM: set your ROM directory in Settings");
            }
            (Some(_), true, Some(path)) => {
                ui.label(format!("✔ ROM: {}", path.display()));
            }
            (Some(dir), ..) => {
                ui.label(format!(
                    "✘ ROM: {}.zip not found in {}",
                    game.rom_name,
                    dir.display()
                ));
            }
        }

        // Wheel
        match wheel::find(&app.settings.wheel_id) {
            Some(w) => {
                ui.label(format!("✔ Controls: preconfigured for {}", w.name));
            }
            None => {
                ui.label("✘ Controls: no wheel profile selected (see Settings)");
            }
        }

        ui.add_space(12.0);

        if let Some(install) = installing {
            let (fraction, text) = match (install.extracting, install.total) {
                (true, _) => (1.0, format!("Extracting {}…", install.current_label)),
                (false, Some(total)) if total > 0 => (
                    install.received as f32 / total as f32,
                    format!(
                        "Downloading {} — {:.1} / {:.1} MB",
                        install.current_label,
                        install.received as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    ),
                ),
                (false, None) => (
                    0.0,
                    format!(
                        "Downloading {} — {:.1} MB",
                        install.current_label,
                        install.received as f64 / 1_048_576.0
                    ),
                ),
                _ => (0.0, String::new()),
            };
            ui.add(egui::ProgressBar::new(fraction).text(text).animate(true));
        } else if !st.emulator_installed && emu.is_some() {
            ui.label("The emulator and force-feedback plugin will be downloaded and configured automatically.");
            ui.add_space(4.0);
            if ui.button("⬇ Download & install emulator").clicked() {
                action = Some(Action::Install(game.emulator_id));
            }
        } else if st.ready() {
            let launch = egui::Button::new(
                egui::RichText::new(format!("🏁 Launch {}", game.title)).size(18.0),
            )
            .min_size(egui::vec2(220.0, 40.0));
            if ui.add(launch).clicked() {
                action = Some(Action::Launch(game));
            }
        }
    });

    action
}
