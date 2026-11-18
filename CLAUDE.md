# IAB-Saturator

Audio saturation plugin (VST3 + CLAP). Learning project: the DSP math is mine,
Claude writes the code (plugin plumbing, GUI, tooling) and reviews.

Read ARCHITECTURE.md before touching code.

## Stack
- Rust, nice-plug (https://codeberg.org/RustAudio/nice-plug, community successor of nih-plug)
- GUI: nice-plug-egui (image-based panel, no stock egui widgets)
- The last nih-plug version (DSP + MVP GUI) is tagged `nih-plug-final`.
- Target: Windows 11, Ableton Live 12

## Commands
- Build bundles: `cargo xtask bundle iab_saturator --release`
- Tests: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Install: copy the .vst3 to `C:\Program Files\Common Files\VST3`
  and the .clap to `C:\Program Files\Common Files\CLAP`

## Rules
- DSP specified in ARCHITECTURE.md is approved: implement it exactly.
  Do NOT invent or change formulas or add DSP blocks without asking first.
- Real-time safety in `process()`: no allocations, no locks, no I/O, no logging, no panics.
- One module per block of the block diagram. Memoryless blocks are pure functions.
  Blocks with state are structs: one instance per channel if they filter audio,
  one shared instance if they smooth a parameter.
- Parameters are smoothed; never apply a raw parameter jump to the audio.
- TDD: tests before logic.
- Code, comments and names in English.

## GUI rules
- The GUI never touches DSP state: it only reads and writes parameters.
- Write parameters only through `ParamSetter` (begin_set / set / end_set), so the
  host records automation and undo.
- Load textures and fonts once when the editor opens, never per frame.
  Allocation is fine on the GUI thread; the real-time rules apply to `process()` only.
- No magic numbers: positions, sizes, colours, angles and texts come from
  `assets/gui/layout.json`; sprite draw sizes are the PNG size / `render_scale`.
- Value readouts use each parameter's own formatter, so they match what Ableton shows.

## GUI assets (`assets/`)
- `gui/layout.json`: the single source of the GUI geometry (840×500 at 1x, `render_scale` 2).
  Generated outside the repo (gen_layout.py); the art is rendered in Blender from it.
- `gui/bg.png` (all static art) + sprites listed in the `assets` block of layout.json,
  2x, straight alpha. Every PNG must be its layout box × `render_scale` (tested).
- `fonts/`: Roboto Slab SemiBold (Apache-2.0) for the live readouts.
- Draw order, angle convention and interaction: see ARCHITECTURE.md › Editor.

## Notation (UPM, not the US convention)
- `fs` sampling rate, `ts` sampling period
- ω: analog angular frequency, Ω: discrete angular frequency (Ω = ω·Ts)
- Drive in dB, linear gain g = 10^(D/20)

## Current scope
- DSP: Drive knob + tanh curve, wired. No oversampling.
- Output, Dry/Wet, Curve, Oversampling and Auto Gain exist as parameters and in the GUI,
  but are NOT wired to the DSP yet. Do not wire them until asked.
- GUI: nice-plug-egui editor from layout.json. VU needles rest until the meters get a
  detector (DSP to be designed).

## Verification
- Sine in → Spectrum after the plugin. Only odd harmonics must appear.
- Compare harmonic levels (dBc) against the values predicted in my notes.
- GUI: every knob position matches the panel scale (−135° at minimum, +135° at maximum).
