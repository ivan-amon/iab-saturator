# IAB-Saturator — Architecture

Mono-curve saturator: one Drive knob into a tanh waveshaper. VST3 + CLAP via nice-plug.

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

Single crate, DSP as modules:

```text
iab_saturator/
├── src/
│   ├── lib.rs               # nice-plug glue: lifecycle, editor creation, calls into dsp/
│   ├── params.rs            # parameters, enums, readout formatters
│   ├── dsp/                 # pure DSP, no framework imports, fully unit-tested
│   │   ├── db.rs            # memoryless dB → linear gain
│   │   ├── drive.rs         # memoryless gain
│   │   ├── curve.rs         # memoryless waveshaper (tanh)
│   │   ├── smoother.rs      # one-pole LPF, has state
│   │   └── harmonics_test.rs # DFT test of drive + curve (test-only)
│   └── editor/              # GUI (nice-plug-egui), reads/writes parameters only
│       ├── mod.rs           # NiceEguiApp: open/close, input then paint each frame
│       ├── layout.rs        # layout.json (serde) + consistency tests vs params
│       ├── assets.rs        # embedded PNGs, decoding + size tests vs layout.json
│       ├── fonts.rs         # embedded Roboto Slab 600
│       ├── geometry.rs      # pure math: rects, angles, drag/wheel → value, text anchor
│       ├── widgets.rs       # input → ParamSetter (knob, selector, segments, toggle)
│       └── paint.rs         # textures + drawing in draw_order (+ headless frame test)
├── assets/
│   ├── gui/                 # layout.json (positions, sizes, colours, texts) + bg.png and
│   │                        # sprites rendered in Blender from it (2x, straight alpha)
│   └── fonts/               # RobotoSlab-SemiBold.ttf (readouts) + licence (Apache-2.0)
└── xtask/                   # nice-plug bundler (nice-plug-xtask)
```

The `dsp` module knows nothing about hosts, threads or where buffers come from.
It only maps samples to samples.

## Parameters

IDs are stored in saved projects and automation: they never change.

| ID             | Name         | Range     | Default   | Unit | Readout     | Status                  |
|----------------|--------------|-----------|-----------|------|-------------|-------------------------|
| `drive`        | Drive        | 0 … +36   | 0         | dB   | `+15.0 dB`  | wired to DSP            |
| `output`       | Output       | −36 … 0   | 0         | dB   | `−6.0 dB`   | declared, not wired yet |
| `dry_wet`      | Dry/Wet      | 0 … 100   | 100       | %    | `75 %`      | declared, not wired yet |
| `curve`        | Curve        | enum (7)  | SOFT CLIP | —    | `SOFT CLIP` | declared, not wired yet |
| `oversampling` | Oversampling | enum (4)  | OFF       | —    | `4×`        | declared, not wired yet |
| `auto_gain`    | Auto Gain    | bool      | off       | —    | —           | declared, not wired yet |

- All ranges are linear.
- Readouts: Drive always signed; Output uses U+2212 (never `−0.0`). Typed values accept
  `+`, `-` or `−`, with or without ` dB`.
- Curve variants (variant ID → display name), in panel order: `tanh` → SOFT CLIP (the tanh
  of the DSP; the old ID is kept so saved projects load), `hard_clip` → HARD CLIP,
  `warm` → WARM, `tube` → TUBE, `fold` → FOLD, `rectify` → RECTIFY, `bit_crush` → BIT CRUSH.
- Oversampling variants: `off` → OFF, `x2` → 2×, `x4` → 4×, `x8` → 8×.
- The GUI changes every parameter; only Drive reaches the audio.
- Output and Dry/Wet get a smoother when they are wired (not before: there is no audio path).

## Editor

- Fixed 840×500 logical px. All images are 2x and drawn at half size.
- Every position, size, colour and text comes from `assets/gui/layout.json`; nothing is
  placed by hand in code. Draw order: `draw_order` in the same file.
- Static art is one image (`bg.png`). Knobs: fixed body sprite + pointer sprite rotated by
  `sweep.from + (sweep.to - sweep.from) * normalized`. The VU needle rotates about its pivot
  and is clipped to the meter face. Readouts are live text (Roboto Slab 600).
- The art is regenerated in Blender from `layout.json` (scripts kept outside the repo).
- The editor reads parameter values and writes them only through `ParamSetter`
  (begin / set / end per gesture). It never touches DSP state.
- Images are decoded once per editor opening and uploaded as textures on the first frame.
- Repaint only when needed. nice-plug-egui does not repaint every frame: it repaints on
  input and when the host reports a parameter or state change (automation, undo, preset).
- VU needles are drawn at `meter_scale.rest_angle` until the meters get a level detector
  (a DSP block, not designed yet).
- Interaction (layout.json `interaction`): knobs drag 200 px = full range, Shift ×0.1,
  double-click = default, wheel = 1 % per notch. CURVE: click = next (wraps), drag/wheel
  step (clamped), click on a label or LED selects it. Oversampling: click a segment.
  Auto Gain: click toggles.
- Not yet in layout.json, so set in code: readout text size = 0.5 × pill height
  (`paint.rs`), wheel step (`geometry.rs`).

## Buffers and channels

- The host calls `process()` once per buffer of N frames (N samples per channel).
- Stereo with N = 8 ⇒ two arrays of 8 samples (L, R), 16 f32 in total.
- N may vary between calls; only the maximum is known at `activate()`.
- Processing order: per frame, then per channel. Time order within each channel
  is always preserved.

## Lifecycle

| Host call      | Thread | What we do                                                        |
|----------------|--------|-------------------------------------------------------------------|
| `activate()`   | main   | Store fs, compute `a = e^(-1/(τ·fs))`, allocate everything        |
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
All memory is allocated in `activate()`. See CLAUDE.md for the full rules.

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