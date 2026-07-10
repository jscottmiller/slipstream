use crate::domain::download::{start_install, InstallEvent, InstallHandle};
use crate::domain::game::GameDef;
use crate::domain::paths::AppPaths;
use crate::domain::settings::Settings;
use crate::domain::{emulator, wheel};
use crate::ui;
use eframe::egui;
use std::collections::HashMap;
use std::time::Duration;

/// An in-flight emulator install, driven by a background thread.
pub(crate) struct InstallState {
    handle: InstallHandle,
    pub current_label: &'static str,
    pub received: u64,
    pub total: Option<u64>,
    pub extracting: bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum View {
    Games,
    Settings,
}

/// Actions the UI requests; applied after drawing to keep borrows simple.
pub(crate) enum Action {
    Install(&'static str),
    Launch(&'static GameDef),
}

pub struct SlipstreamApp {
    pub(crate) paths: AppPaths,
    pub(crate) settings: Settings,
    pub(crate) view: View,
    pub(crate) selected: usize,
    pub(crate) installs: HashMap<&'static str, InstallState>,
    pub(crate) status_line: Option<String>,
}

impl SlipstreamApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let paths = AppPaths::resolve().expect("could not resolve application directories");
        let settings = Settings::load(&paths);
        let status_line = wheel::detect().map(|w| format!("Detected {}", w.name));
        Self {
            paths,
            settings,
            view: View::Games,
            selected: 0,
            installs: HashMap::new(),
            status_line,
        }
    }

    pub(crate) fn save_settings(&mut self) {
        if let Err(e) = self.settings.save(&self.paths) {
            self.status_line = Some(format!("Failed to save settings: {e:#}"));
        }
    }

    fn drain_install_events(&mut self) {
        let mut done: Vec<(&'static str, Option<String>)> = Vec::new();
        for (id, st) in self.installs.iter_mut() {
            while let Ok(event) = st.handle.events.try_recv() {
                match event {
                    InstallEvent::Progress {
                        label,
                        received,
                        total,
                    } => {
                        st.current_label = label;
                        st.received = received;
                        st.total = total;
                        st.extracting = false;
                    }
                    InstallEvent::Extracting { label } => {
                        st.current_label = label;
                        st.extracting = true;
                    }
                    InstallEvent::Finished => done.push((id, None)),
                    InstallEvent::Failed(msg) => done.push((id, Some(msg))),
                }
            }
        }
        for (id, error) in done {
            self.installs.remove(id);
            let name = emulator::find(id).map(|e| e.name()).unwrap_or(id);
            self.status_line = Some(match error {
                None => format!("{name} installed and ready"),
                Some(msg) => format!("Install of {name} failed: {msg}"),
            });
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Install(emulator_id) => {
                let Some(emu) = emulator::find(emulator_id) else {
                    self.status_line = Some(format!("Unknown emulator {emulator_id}"));
                    return;
                };
                let handle = start_install(emu, &self.paths);
                self.installs.insert(
                    emulator_id,
                    InstallState {
                        handle,
                        current_label: "starting…",
                        received: 0,
                        total: None,
                        extracting: false,
                    },
                );
                self.status_line = Some(format!("Installing {}…", emu.name()));
            }
            Action::Launch(game) => self.launch(game),
        }
    }

    fn launch(&mut self, game: &'static GameDef) {
        // Dropping the RunningGame handle is fine — the quit watcher keeps
        // running detached; the desktop UI doesn't track the session.
        let result = crate::domain::launch::launch(game, &self.settings, &self.paths);
        self.status_line = Some(match result {
            Ok(running) => match running.warning {
                Some(warning) => warning,
                None => format!("Launched {} — race on!", game.title),
            },
            Err(e) => format!("Launch failed: {e:#}"),
        });
    }
}

impl eframe::App for SlipstreamApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Alt+Enter hands off to the fullscreen cabinet UI (mirrored there).
        if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::Enter)) {
            match crate::spawn_counterpart(false) {
                Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(e) => self.status_line = Some(format!("Could not open cabinet UI: {e:#}")),
            }
        }

        self.drain_install_events();
        if !self.installs.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("nav").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Slipstream");
                ui.separator();
                ui.selectable_value(&mut self.view, View::Games, "Games");
                ui.selectable_value(&mut self.view, View::Settings, "Settings");
            });
            ui.add_space(4.0);
        });

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.add_space(2.0);
            match &self.status_line {
                Some(line) => ui.label(line.clone()),
                None => ui.weak("Ready"),
            };
            ui.add_space(2.0);
        });

        let action = match self.view {
            View::Games => ui::games::show(self, ui),
            View::Settings => {
                ui::settings::show(self, ui);
                None
            }
        };
        if let Some(action) = action {
            self.apply(action);
        }
    }
}
