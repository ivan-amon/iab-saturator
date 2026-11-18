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
- No magic numbers: sprite sizes come from `assets/gui/sprites.json`,
  positions from the layout table below.
- Value readouts use each parameter's own formatter, so they match what Ableton shows.

## GUI assets (`assets/`)
- `gui/panel.png`: faceplate without knobs, 1200×720 (2x of the 600×360 window).
- `gui/drive.png`, `gui/knob_small.png`, `gui/curve.png`: sprite sheets, row-major grid.
- `gui/screw_tl.png`, `gui/screw_tr.png`, `gui/screw_bl.png`, `gui/screw_br.png`.
- `gui/sprites.json`: frame size (2x), frame count, columns, rows, angle range.
- `fonts/`: Barlow Condensed (OFL) for value readouts.
- Frame for a knob: `k = round(v · (frame_count − 1))`, v = normalized parameter in [0, 1].

## GUI layout (1x px, origin top-left of the 600×360 window)

| Element  | Center (x, y) | Size  | Sprite         | Readout (x, y), color   |
|----------|---------------|-------|----------------|-------------------------|
| DRIVE    | (380, 176)    | Ø112  | drive.png      | (380, 287), #292824     |
| DRY/WET  | (240, 218)    | Ø52   | knob_small.png | (240, 287), #292824     |
| OUTPUT   | (520, 218)    | Ø52   | knob_small.png | (520, 287), #292824     |
| CURVE    | (88, 240)     | Ø40   | curve.png      | (88, 305), #d5cbb1      |
| Screw TL | (20, 20)      | Ø9    | screw_tl.png   | —                       |
| Screw TR | (580, 20)     | Ø9    | screw_tr.png   | —                       |
| Screw BL | (20, 340)     | Ø9    | screw_bl.png   | —                       |
| Screw BR | (580, 340)    | Ø9    | screw_br.png   | —                       |

- Sprites are drawn centered on the element center, at half their pixel size (2x assets).
- Readouts: Barlow Condensed Medium, 12 px, centered on the given point.
- Knob interaction: vertical drag, Shift for fine drag, double-click resets to default,
  mouse wheel steps. CURVE is discrete: drag or click cycles through its 4 positions.

## Notation (UPM, not the US convention)
- `fs` sampling rate, `ts` sampling period
- ω: analog angular frequency, Ω: discrete angular frequency (Ω = ω·Ts)
- Drive in dB, linear gain g = 10^(D/20)

## Current scope
- DSP: Drive knob + tanh curve, wired. No oversampling.
- Output, Dry/Wet and Curve exist as parameters, but are NOT wired
  to the DSP yet. Do not wire them until asked.
- No custom GUI yet (`type Editor = ()`): the host shows its generic parameter view.
  The GUI will be rewritten with nice-plug-egui following the GUI rules and layout above.

## Verification
- Sine in → Spectrum after the plugin. Only odd harmonics must appear.
- Compare harmonic levels (dBc) against the values predicted in my notes.
- GUI: every knob position matches the panel scale (−135° at minimum, +135° at maximum).
