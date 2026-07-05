# Slipstream

A cross-platform launcher for arcade games. Pick a game, click launch — Slipstream
downloads the best emulator for it (explicitly, when you ask), configures video,
controls, and force feedback for your racing wheel, and starts the game.

**v1 scope:** Windows · Daytona USA · Logitech G923. The architecture is
registry-driven so more games, emulators, wheels, and operating systems slot in
without restructuring.

Slipstream **never downloads ROMs**. Point it at a directory containing ROM sets
you own (e.g. `daytona.zip`).

## How it works

- **Games** are entries in a static registry (`src/domain/game.rs`) tying a title
  to a ROM set name and an emulator.
- **Emulators** implement the `Emulator` trait (`src/domain/emulator.rs`):
  download specs (pinned URL + SHA-256), install detection, config generation,
  and launch. v1 ships ElSemi's Model 2 Emulator plus the
  [FFB Arcade Plugin](https://github.com/Boomslangnz/FFBArcadePlugin) for real
  force feedback.
- **Wheels** are profiles (`src/domain/wheel.rs`) describing DirectInput axes,
  buttons, and FFB tuning. The m2emulator integration compiles a profile into
  the emulator's binary `CFG/<rom>.input` control file — no in-emulator
  configuration needed.

### Daytona USA control layout (G923)

| Control | Binding |
| --- | --- |
| Steering / pedals | Wheel, throttle, brake |
| Gears 1–4 | D-pad up / right / down / left |
| VR view buttons | Cross, Square, Circle, Triangle |
| Start | Options |
| Insert coin | Share |

## Building

Native (Linux dev):

```sh
cargo build && cargo test
```

Windows release build (from WSL, needs `mingw-w64` and the
`x86_64-pc-windows-gnu` target — both declared in `rust-toolchain.toml` /
`.cargo/config.toml`):

```sh
cargo build --release --target x86_64-pc-windows-gnu
```

The exe lands at `target/x86_64-pc-windows-gnu/release/slipstream.exe` and is
fully self-contained.

## Data locations (Windows)

- Settings: `%APPDATA%\cowboyscott\slipstream\config\config.toml`
- Emulators: `%LOCALAPPDATA%\cowboyscott\slipstream\data\emulators\`
- Download cache: `%LOCALAPPDATA%\cowboyscott\slipstream\data\downloads\`
