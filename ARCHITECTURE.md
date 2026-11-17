# IAB-Saturator — Architecture

Mono-curve saturator: one Drive knob into a tanh waveshaper. VST3 + CLAP via nih-plug.

## Signal flow

```text
                    D (dB)  ← knob (atomic, read in process())
                      │
                      ▼
               ┌─────────────┐
               │ g=10^(D/20) │   dB → linear
               └──────┬──────┘
                      │ g[n]   (staircase)
                      ▼
               ┌─────────────┐
               │  Smoother   │   one-pole LPF, has state, shared by all channels
               │             │   gs[n] = (1-a)·g[n] + a·gs[n-1]
               │             │   a = e^(-1/(τ·fs))
               └──────┬──────┘
                      │ gs[n]  (smooth)
                      ▼
 x[n]          ┌─────────────┐   v[n]    ┌─────────────┐    y[n]
──────────────►│    Drive    ├──────────►│    Curve    ├──────────►
 (per channel) │ v = gs · x  │           │ y = tanh(v) │  (per channel)
               └─────────────┘           └─────────────┘
                memoryless                 memoryless
                (product of 2 signals)     (static nonlinearity)
```

## Crate layout

Single crate (the nih-plug template), DSP as modules:

```text
iab_saturator/
├── src/
│   ├── lib.rs               # nih-plug glue: params, lifecycle, calls into dsp/
│   ├── dsp/                 # pure DSP, no nih_plug imports, fully unit-tested
│   │   ├── db.rs            # memoryless dB → linear gain
│   │   ├── drive.rs         # memoryless gain
│   │   ├── curve.rs         # memoryless waveshaper (tanh)
│   │   ├── smoother.rs      # one-pole LPF, has state
│   │   └── harmonics_test.rs # DFT test of drive + curve (test-only)
│   └── editor/              # GUI (egui), reads/writes parameters only
│       ├── mod.rs           # per-window UI state, repaint policy
│       ├── window.rs        # nih-plug Editor: egui-baseview window (adapted from nih_plug_egui)
│       ├── layout.rs        # the layout table from CLAUDE.md
│       ├── sprites.rs       # sprites.json, frame index, UV rect
│       ├── assets.rs        # embedded PNGs, decoded when the editor opens
│       ├── fonts.rs         # embedded Barlow Condensed Medium
│       ├── paint.rs         # textures + drawing (panel, screws, knobs, readouts)
│       ├── knob.rs          # knob input: drag / Shift / double-click / wheel
│       ├── curve_selector.rs # CURVE input: click / drag / double-click
│       ├── alignment_test.rs # pixel checks: sprite pointers vs panel ticks (test-only)
│       └── preview_test.rs  # software render of paint() to PNG (test-only)
├── assets/                  # gui/ (panel, sprite sheets, sprites.json), fonts/ (OFL)
└── xtask/                   # nih-plug bundler
```

The `dsp` module knows nothing about hosts, threads or where buffers come from.
It only maps samples to samples.

## Parameters

IDs are stored in saved projects and automation: they never change.

| ID        | Name    | Range          | Default | Unit | Readout     | Status                  |
|-----------|---------|----------------|---------|------|-------------|-------------------------|
| `drive`   | Drive   | 0 … +36        | 0       | dB   | `15.0 dB`   | wired to DSP            |
| `output`  | Output  | −36 … 0        | 0       | dB   | `-6.0 dB`   | declared, not wired yet |
| `dry_wet` | Dry/Wet | 0 … 100        | 100     | %    | `75 %`      | declared, not wired yet |
| `curve`   | Curve   | enum (4)       | Tanh    | —    | `TANH`      | declared, not wired yet |

- All ranges are linear.
- Curve variants (variant ID → display name): `tanh` → TANH, `curve2` → CURVE 2,
  `curve3` → CURVE 3, `curve4` → CURVE 4.
- Output and Dry/Wet get a smoother when they are wired (not before: there is no audio path).
- The editor window size is persisted in the plugin state (`editor-state`).

## Editor

- Fixed 600×360 logical px. All images are 2x and drawn at half size.
- The editor reads parameter values and writes them only through `ParamSetter`
  (begin / set / end per gesture). It never touches DSP state.
- Images are decoded once per editor opening and uploaded as textures on the first frame.
- Repaint policy: egui runs every frame (cheap), but the window renders only when a
  displayed value changed (GUI input, host automation, undo, preset), on input, and during
  the first 30 frames after opening.

## Buffers and channels

- The host calls `process()` once per buffer of N frames (N samples per channel).
- Stereo with N = 8 ⇒ two arrays of 8 samples (L, R), 16 f32 in total.
- N may vary between calls; only the maximum is known at `initialize()`.
- Processing order: per frame, then per channel. Time order within each channel
  is always preserved.

## Lifecycle

| Host call      | Thread | What we do                                                        |
|----------------|--------|-------------------------------------------------------------------|
| `initialize()` | main   | Store fs, compute `a = e^(-1/(τ·fs))`, allocate everything        |
| `reset()`      | audio  | Reset smoother state (snap gs to current g)                       |
| `process()`    | audio  | Per sample: gs = smoother.next(g); per channel: y = tanh(gs · x)  |

## State

- Smoother: one shared instance (Drive is a single parameter for both channels),
  persists across buffers.
- Drive and Curve: stateless (pure functions), applied to each channel.
- Any future block that filters audio (oversampling, emphasis filters):
  one instance per channel.

## Real-time constraints

`process()` must finish within N/fs. Inside it: no allocation or deallocation,
no locks, no I/O or logging, no panics, bounded work only.
All memory is allocated in `initialize()`. See CLAUDE.md for the full rules.

## Expected behaviour (verification)

- Odd curve ⇒ only odd harmonics (f0, 3f0, 5f0, …) for a sine input.
- Small signal (|g·x| ≪ 1): transparent, y ≈ g·x.
- Large signal: tends to a square wave, a_k = 2/(πk)·(-1)^((k-1)/2).
- Harmonic levels (dBc) must match the reference values in the notes.
- Moving Drive must not produce clicks or zipper noise.

## Known limitations

- No oversampling: harmonics above fs/2 alias. Audible with high Drive and high f0.
- No output gain: loudness rises with Drive until the curve limits it.
- Channels processed independently (no stereo linking).

## Roadmap

1. Oversampling around the curve (report latency to host).
2. Output gain / auto gain compensation.
3. Asymmetric curve (bias) ⇒ even harmonics.
4. Pre/de-emphasis filters around the curve (from eq-rs).