# Slipstream

An arcade launcher for racing games. Pick a game, click **Launch** — Slipstream
downloads the right emulator (only when you ask), generates its video and
control configuration for your wheel, sets up force feedback, and starts the
game. No emulator menus, no binary config files, no test-mode dip-switch
archaeology.

**Current scope (v0.2):** Windows · Logitech G923 · seven games across two
systems — Daytona USA and Sega Rally Championship (Sega Model 2, via ElSemi's
Model 2 Emulator) and Scud Race, Daytona USA 2 (both editions), Sega Rally 2,
and Le Mans 24 (Sega Model 3, via
[Supermodel](https://github.com/trzy/Supermodel)). The design is
registry-driven — games, emulators, wheels, and platforms are pluggable — so
the scope grows without restructuring.

Slipstream **never downloads ROMs**. You point it at a directory containing
ROM sets you own.

## What "turnkey" means here

Launching Daytona USA on a modern PC normally involves: finding the emulator,
finding the force-feedback plugin, clicking through a binary control-config
dialog, discovering the game boot-loops with `NETWORK BOARD NOT PRESENT`
because its battery-backed SRAM defaults to linked-cabinet mode, and learning
which of your USB devices is secretly a phantom game controller. Slipstream
does all of that in code, on every launch:

- **`EMULATOR.INI`** — video mode, ROM paths, force-feedback flags. Owned by
  Slipstream and regenerated per launch.
- **`CFG/<rom>.input`** — the emulator's binary per-game control file,
  compiled from a wheel profile (axes, buttons, d-pad, inversion, pad index).
  The G923 layout was captured from the emulator's own config dialog on real
  hardware and is golden-tested byte-for-byte.
- **`NVDATA/<rom>.DAT`** — game backup RAM, seeded with a single-cabinet
  configuration so link-mode defaults can't demand the cabinet-link network
  board. Repaired only when wrong; your calibration and high scores survive.
- **Force feedback** — per-wheel strategy. The G923 uses the emulator's
  native DirectInput effects (the actual arcade drive-board commands:
  centering spring, clutch friction, roll forces). The
  [FFB Arcade Plugin](https://github.com/Boomslangnz/FFBArcadePlugin) is
  installed alongside for wheels that need it, parked as
  `dinput8.dll.disabled` when unused.

## Quick start

1. Grab `slipstream.exe` (or build it — see below) and run it.
2. **Settings** → set your ROM directory (must contain `daytona.zip`; split
   sets also need `model2.zip` beside it) and confirm your wheel.
3. **Games** → Daytona USA → **Download & install emulator** (fetches ElSemi's
   Model 2 Emulator and the FFB plugin, SHA-256 verified, with progress).
4. **Launch.** Menu button starts, View inserts a coin, X/A/Y/B are gears 1–4,
   pedals and wheel do what pedals and wheels do.

### Daytona USA on a G923 (Xbox/PC)

| Control | Binding |
| --- | --- |
| Steering / throttle / brake | Wheel and pedals |
| Gears 1–4 | X, A, Y, B |
| VR view buttons | Rear wheel buttons and paddles |
| Start | Menu |
| Insert coin | View |
| Menu navigation | D-pad |

If another game controller enumerates ahead of your wheel (some Razer
keyboards register a phantom gamepad), raise **Settings → Controller number**
or disable the phantom device in Device Manager.

## Portable mode

A `config.toml` next to `slipstream.exe` makes the folder self-contained:
`emulators/`, `downloads/`, and (by convention) `roms/` live beside the exe,
and a relative `rom_dir` resolves against it. Move or copy the folder freely.

Without a local config, Slipstream uses the platform directories:
`%APPDATA%` / `%LOCALAPPDATA%\cowboyscott\slipstream` on Windows.

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
- `src/emulators/m2/` — Model 2 Emulator integration: INI writer, binary
  `.input` compiler, NVRAM seeding, FFB plugin management
- `src/domain/download.rs` — background installs with streaming SHA-256
  verification and zip/7z extraction

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
  project, whose generator cross-checked the layout.

## Legal

Slipstream downloads freely distributed emulator binaries from their public
mirrors and verifies them by checksum. It does not include or download game
ROMs; you must own the games you play. Daytona USA and Sega Model 2 are
trademarks of SEGA. This project is not affiliated with SEGA or Logitech.

## License

[MIT](LICENSE)
