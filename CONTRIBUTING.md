# Contributing to Slipstream

Contributions welcome — new games, wheels, and emulator integrations are all
designed to be added without restructuring. A few ground rules, then the
recipes.

## Ground rules

- **No ROM content, ever.** No ROM files, no download links to ROMs, no
  workarounds. Emulator binaries are fine when they're freely distributed —
  pinned to an exact release URL with a SHA-256 hash in the `DownloadSpec`.
- **Defaults must be hardware-verified where possible.** The best control
  config is one captured from the emulator's own configuration tool on real
  hardware, then encoded as a generator plus a golden test (see below).
  Plausible-from-documentation is a starting point; say so in the PR if
  that's all you have.
- **Green checks**: `cargo test`, `cargo clippy --all-targets` (zero
  warnings), and the Windows cross-build
  (`cargo build --release --target x86_64-pc-windows-gnu`) must all pass.
- Keep code in the style of what's around it. The codebase favors small
  modules, plain data registries, and generators with tests over runtime
  cleverness.

## Dev setup

Linux/WSL: stable Rust (see `rust-toolchain.toml`), plus `mingw-w64` for the
Windows cross-build. `cargo build && cargo test` for the host; the release
target is `x86_64-pc-windows-gnu`.

## Adding a wheel

Wheels are static `WheelProfile` entries in `src/domain/wheel.rs`:

1. **Identify the hardware**: USB vendor/product ids (Device Manager, or any
   HID lister). Add every product id variant the wheel presents. These drive
   auto-detection; detection is best-effort and users can always select the
   profile manually.
2. **Discover the DirectInput layout** — axes for steering/throttle/brake
   (and whether pedals rest at maximum: that's `inverted: true`), plus
   1-based button numbers. Two reliable ways:
   - Run `supermodel -config-inputs` in a Supermodel install, bind
     everything from the wheel, and read `Config/Supermodel.ini` — the
     mapping strings name the axes and buttons directly.
   - Or bind controls in m2emulator's own dialog (Game → Configure Controls)
     and decode `CFG/<rom>.input` — the format is documented at the top of
     `src/emulators/m2/input_file.rs`.
3. **Fill the profile**: axis bindings, gear controls (buttons or d-pad),
   start/coin, the four VR buttons, the console button (`btn_quit` — the
   Xbox/PS logo, which quits back to the launcher; `None` if not yet
   captured), and `ffb_mode`. Start with
   `FfbMode::EmulatorNative` — it's verified on the G923 and needs no extra
   moving parts; `FfbMode::Plugin` activates the FFB Arcade Plugin instead.
4. **Add the profile to `WHEELS`** and extend the golden tests: for the m2
   integration that means expected `.input` bytes for your wheel (compute
   them from the encoding doc, or better, capture the emulator's own output
   and assert equality — see `daytona_g923_xbox_matches_hardware_capture`).

The profile's semantics are emulator-agnostic; each integration translates
them (`inverted` becomes an invert flag in m2's binary format but a `_NEG`
half-axis in Supermodel's DSL — the writers own that knowledge, not the
profile).

## Adding a game

1. **Registry entry** in `src/domain/game.rs`: id, title, year, manufacturer,
   system, MAME-style parent ROM set name, and which emulator runs it.
2. **Control layout**, depending on the emulator:
   - **Supermodel**: usually nothing to do — driving inputs are declared
     globally in the generated ini and Supermodel ignores what a game doesn't
     use. Only touch `src/emulators/supermodel/ini.rs` if the game needs an
     input the generator doesn't emit yet.
   - **Model 2 Emulator**: add a layout function in
     `src/emulators/m2/input_file.rs`. The per-game input slot order must be
     discovered empirically: configure the game once in the emulator's dialog
     and decode the resulting `.input` file (RetroBat's
     `Model2.Controllers.cs` is a useful cross-check). Reuse
     `driving_common` for the standard racing prefix, and add a golden test.
3. **Check for boot traps.** Sega's linked-cabinet racers (both Daytonas so
   far) ship NVRAM defaults that demand the cabinet network board and
   boot-loop with `NETWORK BOARD NOT PRESENT`. The fix: set Game Assignments
   → Link ID → SINGLE in the test menu (F2; F1 moves the cursor), exit
   cleanly, then capture the NVRAM file the emulator wrote and embed it —
   see `src/emulators/m2/nvram.rs` and `src/emulators/supermodel/nvram.rs`
   for the pattern. Supermodel's `.nv` container embeds the game name and
   emulator version, so images are per-game and per-pinned-release.
4. **Verify the ROM set** you're testing with against the emulator's own
   expectations (Supermodel's `Config/Games.xml`, or the Model 2 Emulator
   DAT). Watch for split sets (shared files in a companion zip, like
   `model2.zip`) and pre-rename MAME file names — document any such
   requirement in the README's games table.
5. **Update the README** games table, and test on hardware before calling
   the defaults verified.

## Adding an emulator

Implement the `Emulator` trait (`src/domain/emulator.rs`) in a new module
under `src/emulators/`, and register it in `EMULATORS`:

- `downloads()` — pinned release URL(s) + SHA-256; the download manager
  handles zip and 7z, whole-archive or a single subdirectory.
- `is_installed()` — cheap filesystem check.
- `configure()` — write *all* configuration (video, controls, FFB, NVRAM
  seeds) from the wheel profile and settings. Slipstream owns emulator
  config: it's regenerated on every launch, so generators must be complete
  and deterministic. Prefer plain-text formats where the emulator offers a
  choice; write binary formats with a documented encoder and golden tests.
- `launch()` — spawn with the working directory the emulator expects.
- `needs_escape_quit()` — return true if the emulator has no quit key of its
  own; the launcher's quit watcher then closes it gracefully on Escape or
  the wheel's console button (emulators that quit on Escape natively get the
  console button translated into an Escape press instead).

## PR checklist

- [ ] `cargo test` and `cargo clippy --all-targets` clean
- [ ] Windows cross-build compiles
- [ ] Golden tests for any binary format changes
- [ ] README games/controls tables updated if applicable
- [ ] PR notes say what was hardware-verified vs. documentation-derived
- [ ] No ROM content or links
