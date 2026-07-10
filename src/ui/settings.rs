use crate::app::SlipstreamApp;
use crate::domain::settings::DefaultUi;
use crate::domain::wheel;
use eframe::egui;

pub(crate) fn show(app: &mut SlipstreamApp, ui: &mut egui::Ui) {
    let mut dirty = false;

    egui::CentralPanel::default().show(ui, |ui| {
        ui.add_space(8.0);
        ui.heading("Settings");
        ui.add_space(8.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("ROMs").strong());
            ui.label("Slipstream never downloads ROMs — point it at your own collection.");
            ui.horizontal(|ui| {
                let text = app
                    .settings
                    .rom_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(not set)".to_string());
                ui.monospace(text);
                if ui.button("Browse…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        app.settings.rom_dir = Some(dir);
                        dirty = true;
                    }
                }
            });
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("Wheel").strong());
            ui.horizontal(|ui| {
                let current = wheel::find(&app.settings.wheel_id)
                    .map(|w| w.name)
                    .unwrap_or("(none)");
                egui::ComboBox::from_id_salt("wheel_select")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for w in wheel::WHEELS {
                            if ui
                                .selectable_value(
                                    &mut app.settings.wheel_id,
                                    w.id.to_string(),
                                    w.name,
                                )
                                .changed()
                            {
                                dirty = true;
                            }
                        }
                    });
                if ui.button("Detect connected wheel").clicked() {
                    app.status_line = Some(match wheel::detect() {
                        Some(w) => {
                            if app.settings.wheel_id != w.id {
                                app.settings.wheel_id = w.id.to_string();
                                dirty = true;
                            }
                            format!("Detected {}", w.name)
                        }
                        None => "No known wheel detected".to_string(),
                    });
                }
            });
            ui.horizontal(|ui| {
                ui.label("Controller number");
                if ui
                    .add(egui::DragValue::new(&mut app.settings.wheel_pad).range(1..=8))
                    .changed()
                {
                    dirty = true;
                }
                ui.weak("(raise if other game controllers enumerate before the wheel)");
            });
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("Interface").strong());
            ui.horizontal(|ui| {
                ui.label("Start in");
                for (value, label) in [
                    (DefaultUi::Cabinet, "Cabinet (fullscreen)"),
                    (DefaultUi::Desktop, "Desktop"),
                ] {
                    if ui
                        .selectable_value(&mut app.settings.default_ui, value, label)
                        .changed()
                    {
                        dirty = true;
                    }
                }
                ui.weak("(Alt+Enter switches anytime)");
            });
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("Video").strong());
            if ui
                .checkbox(&mut app.settings.fullscreen, "Fullscreen")
                .changed()
            {
                dirty = true;
            }
            ui.horizontal(|ui| {
                ui.label("Resolution");
                if ui
                    .add(
                        egui::DragValue::new(&mut app.settings.screen_width)
                            .range(640..=7680)
                            .speed(10),
                    )
                    .changed()
                {
                    dirty = true;
                }
                ui.label("×");
                if ui
                    .add(
                        egui::DragValue::new(&mut app.settings.screen_height)
                            .range(480..=4320)
                            .speed(10),
                    )
                    .changed()
                {
                    dirty = true;
                }
            });
        });
    });

    if dirty {
        app.save_settings();
    }
}
