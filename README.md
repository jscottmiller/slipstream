# Slipstream

An arcade launcher for racing games. Pick a game, click **Launch** — Slipstream
downloads the right emulator (only when you ask), generates video, controls,
and force feedback for your wheel, and starts the game. Escape quits, every
game, every emulator. No config dialogs, no test-menu archaeology.

Slipstream **never downloads ROMs**. You point it at a directory containing
ROM sets you own.

## Games (v0.2)

Windows · Logitech G923. Seven games across two systems:

| Game | System | Emulator | ROM set(s) needed |
| --- | --- | --- | --- |
| Daytona USA | Model 2 | Model 2 Emulator | `daytona.zip` + `model2.zip` |
| Sega Rally Championship | Model 2 | Model 2 Emulator | `srallyc.zip` + `model2.zip` |
| Scud Race | Model 3 | Supermodel | `scud.zip` |
| Le Mans 24 | Model 3 | Supermodel | `lemans24.zip` |
| Daytona USA 2: Battle on the Edge | Model 3 | Supermodel | `daytona2.zip` |
| Daytona USA 2: Power Edition | Model 3 | Supermodel | `dayto2pe.zip` |
| Sega Rally 2 | Model 3 | Supermodel | `srally2.zip` |

MAME-style ROM sets. `model2.zip` carries the Model 2 games' shared TGP table
ROMs; the Model 3 sets include their drive-board ROMs, which power force
feedback. The design is registry-driven — games, emulators, wheels, and
platforms are pluggable.

## Quick start

1. Run `slipstream.exe` (or build it — see below).
2. **Settings** → set your ROM directory and confirm your wheel.
3. Pick a game → **Download & install emulator** (SHA-256-pinned, with
   progress; each emulator installs once and serves all its games).
4. **Launch.** **Escape quits back to the launcher.**

## Controls (G923 Xbox/PC)

| Control | Binding |
| --- | --- |
| Steering / throttle / brake | Wheel and pedals |
| Gears 1–4 | X, A, Y, B |
| Sequential shift (Model 3) | Paddles |
| VR view buttons | Rear wheel buttons and paddles |
| View change / handbrake (Sega Rally 2) | Rear wheel buttons |
| Start | Menu |
| Insert coin | View |
| Menu navigation | D-pad |

Keyboard fallbacks work too (1 = start, 5 = coin, F2 = test menu). If another
game controller enumerates ahead of your wheel (some Razer keyboards register
a phantom gamepad), raise **Settings → Controller number** or disable the
phantom in Device Manager.

## Under the hood

Slipstream owns the emulator configuration and regenerates it on every launch:

- **Model 2 Emulator**: `EMULATOR.INI` (video, ROM paths, FFB) plus the binary
  `CFG/<rom>.input` control file, compiled from the wheel profile and
  golden-tested byte-for-byte against a capture from the emulator's own
  config dialog on real hardware.
- **Supermodel**: `Config/Supermodel.ini` in its text mapping DSL. Pedals that
  rest at axis maximum are expressed as negative half-axes (`JOY1_YAXIS_NEG`)
  — the encoding Supermodel's own `-config-inputs` produces.
- **NVRAM seeding**: both Daytonas ship factory defaults set for linked
  cabinets and boot-loop with `NETWORK BOARD NOT PRESENT`. Slipstream seeds
  hardware-captured single-cabinet backup RAM (and for Model 2, repairs a
  linked config in place). High scores and calibration survive.
- **Force feedback**: per-wheel strategy. The G923 uses each emulator's native
  DirectInput effects — the real arcade drive-board commands. The
  [FFB Arcade Plugin](https://github.com/Boomslangnz/FFBArcadePlugin) is
  installed alongside for wheels that need it, parked as
  `dinput8.dll.disabled` when unused (its SDL haptic path fails silently on
  the G923).
- **Quit on Escape**: Supermodel does this natively; m2emulator has no quit
  key, so the launcher watches the spawned process and sends a graceful
  window close when Escape is pressed with the emulator in the foreground —
  NVRAM still flushes on exit.

## Portable mode

A `config.toml` next to `slipstream.exe` makes the folder self-contained:
`emulators/`, `downloads/`, and (by convention) `roms/` live beside the exe,
and a relative `rom_dir` resolves against it. Move or copy the folder freely
— between machines, onto the HTPC, wherever.

Without a local config, Slipstream uses platform directories
(`%APPDATA%` / `%LOCALAPPDATA%\cowboyscott\slipstream` on Windows).

## Building

Linux/WSL development:

```sh
cargo build && cargo test
```

Windows release (cross-compiled from WSL; needs `mingw-w64`, and
`rust-toolchain.toml` pulls the `x86_64-pc-windows-gnu` target):

```sh
cargo build --release --target x86_64-pc-windows-gnu
```

The result is a single self-contained exe.

## Architecture

- `src/domain/game.rs` — game registry: title → ROM set → emulator
- `src/domain/emulator.rs` — `Emulator` trait: pinned download specs
  (URL + SHA-256), install detection, config generation, launch
- `src/domain/wheel.rs` — wheel profiles: DirectInput axes/buttons, gear
  layout, FFB strategy, USB ids for HID auto-detection
- `src/domain/download.rs` — background installs with streaming SHA-256
  verification and zip/7z extraction
- `src/domain/quit_watcher.rs` — Escape-to-quit for emulators without a
  quit key
- `src/emulators/m2/` — Model 2 Emulator: INI writer, binary `.input`
  compiler, NVRAM seeding/repair, FFB plugin management
- `src/emulators/supermodel/` — Supermodel: text config generation, NVRAM
  seeding, command-line launch

## Credits

- **ElSemi** — the Model 2 Emulator, still the definitive way to play these
  games.
- **Bart Trzynadlowski & the Supermodel team** — the
  [Supermodel](https://github.com/trzy/Supermodel) Sega Model 3 emulator,
  whose plain-text config and built-in force feedback made this integration
  a pleasure.
- **Boomslangnz & contributors** — the
  [FFB Arcade Plugin](https://github.com/Boomslangnz/FFBArcadePlugin).
- **SkylarZYX** — the single-cabinet Daytona NVRAM preset (from
  [daytona-usa-script-utils](https://github.com/MikroMacaroni/daytona-usa-script-utils)).
- **andersstorhaug** — the original
  [`.input` format notes](https://gist.github.com/andersstorhaug/38ceadbae32790c08c8b130cb4a8486b),
  and the [RetroBat](https://github.com/RetroBat-Official/emulatorlauncher)
  project, whose generators cross-checked both integrations.

## Legal

Slipstream downloads freely distributed emulator binaries from their official
releases and public mirrors, verified by checksum. It does not include or
download game ROMs; you must own the games you play. Daytona USA, Sega Rally,
Scud Race, and Sega Model 2/3 are trademarks of SEGA. This project is not
affiliated with SEGA or Logitech.

## License

[MIT](LICENSE)
