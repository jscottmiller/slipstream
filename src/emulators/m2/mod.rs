//! ElSemi's Model 2 Emulator ("m2emulator") — the definitive Sega Model 2
//! emulator. Windows-only, closed-source freeware, frozen at v1.1a (2014).
//! Force feedback comes from the FFB Arcade Plugin (dinput8.dll hook), which
//! reads game memory for real per-game effects.

pub mod ffb_plugin;
pub mod ini;
pub mod input_file;
pub mod nvram;

use crate::domain::emulator::{ArchiveKind, DownloadSpec, Emulator, ExtractRule};
use crate::domain::game::{ControlKind, GameDef};
use crate::domain::paths::AppPaths;
use crate::domain::settings::Settings;
use crate::domain::wheel::{FfbMode, WheelProfile};
use anyhow::{bail, Context, Result};
use std::process::{Child, Command};

pub const EXE_NAME: &str = "emulator_multicpu.exe";

pub struct M2Emulator;

static DOWNLOADS: &[DownloadSpec] = &[
    DownloadSpec {
        label: "Model 2 Emulator 1.1a",
        // The official site (nebula.emulatronia.com) is offline; this is the
        // emulator-only zip mirrored on the Internet Archive.
        url: "https://archive.org/download/m2emu1.1a/Sega%20Model%202%20Emu%201.1a%20and%20Full%20Romset/Model%202%20Emulator%201.1a/m2emulator.zip",
        sha256: "5ffebe72d2885bde3fbfab816947475a9a2ce2795284b9d1e90344209bd4c65f",
        kind: ArchiveKind::Zip,
        extract: ExtractRule::All,
    },
    DownloadSpec {
        label: "FFB Arcade Plugin 2.0.0.53",
        url: "https://github.com/Boomslangnz/FFBArcadePlugin/releases/download/2.0.0.53/FFB.Arcade.Plugin.v2.0.0.53.7z",
        sha256: "c972e9b2802e1a35d4532646ea728aa4802f72a751fd958a77f8fda2f31ce27d",
        kind: ArchiveKind::SevenZ,
        extract: ExtractRule::Subdir("M2Emulator"),
    },
    // Lightgun companion: hooks the emulator and writes gun coordinates
    // (including true offscreen values) straight into game memory — the
    // only way to get offscreen reload, since a window-bound mouse can
    // never produce out-of-bounds coordinates once a game is calibrated.
    DownloadSpec {
        label: "DemulShooter v17.5",
        url: "https://github.com/argonlefou/DemulShooter/releases/download/v17.5/DemulShooter_v17.5.zip",
        sha256: "792f03fcde4e827b82ca2dfad2a5d4a25316c53fe219eff77398e57d239eeba7",
        kind: ArchiveKind::Zip,
        extract: ExtractRule::All,
    },
];

const DEMULSHOOTER_EXE: &str = "DemulShooter.exe";

impl Emulator for M2Emulator {
    fn id(&self) -> &'static str {
        "m2"
    }

    fn name(&self) -> &'static str {
        "Model 2 Emulator (ElSemi)"
    }

    fn downloads(&self) -> &'static [DownloadSpec] {
        DOWNLOADS
    }

    fn is_installed(&self, paths: &AppPaths) -> bool {
        let dir = self.install_dir(paths);
        // The FFB plugin hook may be parked as dinput8.dll.disabled when a
        // wheel profile uses the emulator's native force feedback.
        // DemulShooter arrived later: older installs show as not-installed
        // so the desktop UI offers the (idempotent) re-install that adds it.
        dir.join(EXE_NAME).exists()
            && (dir.join("dinput8.dll").exists() || dir.join("dinput8.dll.disabled").exists())
            && dir.join(DEMULSHOOTER_EXE).exists()
    }

    fn configure(
        &self,
        game: &GameDef,
        settings: &Settings,
        wheel: &WheelProfile,
        paths: &AppPaths,
    ) -> Result<()> {
        let dir = self.install_dir(paths);
        let rom_dir = settings
            .rom_dir
            .as_ref()
            .context("ROM directory is not set (Settings → ROM directory)")?;

        let native_ffb = wheel.ffb_mode == FfbMode::EmulatorNative;
        let lightgun = game.controls == ControlKind::Lightgun;
        std::fs::write(
            dir.join("EMULATOR.INI"),
            ini::emulator_ini(rom_dir, settings, native_ffb, lightgun),
        )
        .context("writing EMULATOR.INI")?;

        match game.controls {
            ControlKind::Wheel => {
                let controls = input_file::for_game(game, wheel, settings.wheel_pad)
                    .with_context(|| format!("no control layout for {}", game.id))?;
                let cfg_dir = dir.join("CFG");
                std::fs::create_dir_all(&cfg_dir)?;
                std::fs::write(cfg_dir.join(format!("{}.input", game.rom_name)), controls)
                    .context("writing control config")?;
            }
            // Lightgun games ride m2emulator's defaults, which already aim
            // with the mouse (Gun4IR in mouse mode) — and the mouse `.input`
            // encoding hasn't been captured yet. An existing `.input` is
            // left alone so in-emulator gun calibration survives launches.
            ControlKind::Lightgun => {}
        }

        match wheel.ffb_mode {
            FfbMode::EmulatorNative => ffb_plugin::set_active(&dir, false)?,
            FfbMode::Plugin => {
                ffb_plugin::set_active(&dir, true)?;
                ffb_plugin::apply(&dir, wheel)?;
            }
        }

        // Seed/repair backup RAM so link-mode defaults can't demand the
        // cabinet-link network board and boot-loop the game.
        if let Some(image) = nvram::for_game(game.id) {
            nvram::ensure_single_link(&dir.join("NVDATA"), game.rom_name, image)
                .context("preparing NVRAM")?;
        }
        // Gun games get hardware-captured gun calibration on first run.
        if let Some(image) = nvram::calibration_seed(game.id) {
            nvram::seed_if_missing(&dir.join("NVDATA"), game.rom_name, image)
                .context("seeding gun calibration")?;
        }
        Ok(())
    }

    // Model 2 games split shared TGP table ROMs into a companion set.
    fn required_rom_sets(&self, game: &GameDef) -> Vec<&'static str> {
        vec![game.rom_name, "model2"]
    }

    /// DemulShooter for gun games: it waits for the emulator process and
    /// hooks it, feeding raw gun coordinates directly — offscreen reload
    /// included. Gun devices are bound once per machine via its own
    /// DemulShooter_GUI.exe; absent or unconfigured it just does nothing
    /// and the game falls back to plain mouse aim. Slipstream spawns it at
    /// the same privilege level as the emulator, so no elevation is needed
    /// despite DemulShooter's blanket run-as-admin guidance (hardware-
    /// verified unelevated; the guidance targets elevated emulators).
    fn launch_companions(&self, game: &GameDef, paths: &AppPaths) -> Vec<Child> {
        if game.controls != ControlKind::Lightgun {
            return Vec::new();
        }
        let dir = self.install_dir(paths);
        let exe = dir.join(DEMULSHOOTER_EXE);
        if !exe.exists() {
            log::warn!("DemulShooter not installed; offscreen reload unavailable");
            return Vec::new();
        }
        match Command::new(&exe)
            .arg("-target=model2")
            .arg(format!("-rom={}", game.rom_name))
            .current_dir(&dir)
            .spawn()
        {
            Ok(child) => vec![child],
            Err(e) => {
                log::warn!("starting DemulShooter: {e}");
                Vec::new()
            }
        }
    }

    // m2emulator's own ESC only toggles fullscreen; there is no quit key.
    fn needs_escape_quit(&self) -> bool {
        true
    }

    fn launch(&self, game: &GameDef, _settings: &Settings, paths: &AppPaths) -> Result<Child> {
        let dir = self.install_dir(paths);
        let exe = dir.join(EXE_NAME);
        if !exe.exists() {
            bail!("{} is not installed", self.name());
        }
        // Working dir must be the emulator dir so it finds its INI, CFG and
        // the dinput8.dll FFB hook.
        Command::new(&exe)
            .arg(game.rom_name)
            .current_dir(&dir)
            .spawn()
            .context("failed to start emulator")
    }
}
