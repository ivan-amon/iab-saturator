# IAB-Saturator — Architecture

Mono-curve saturator: Drive into a tanh waveshaper, then Dry/Wet mix and Output gain.
VST3 + CLAP via nice-plug.

## Signal flow

### Control path (every continuous knob)

Every knob that reaches the audio follows the same path, with its own `Smoother` instance
(same τ for all). The audio path only ever sees the smoothed value ps[n].

```text
                      P  ← knob (atomic, read once per buffer in process())
                      │
                      ▼
               ┌─────────────┐
               │   Convert   │   memoryless: to the unit the audio block expects
               └──────┬──────┘
                      │ p[n]   (staircase)
                      ▼
               ┌─────────────┐
               │  Smoother   │   one-pole LPF, has state, shared by all channels
               │             │   ps[n] = (1-a)·p[n] + a·ps[n-1]
               │             │   a = e^(-1/(τ·fs))
               └──────┬──────┘
                      │ ps[n]  (smooth)
                      ▼
```

| Knob    | P      | Convert            | ps[n] (name in `lib.rs`) |
|---------|--------|--------------------|--------------------------|
| Drive   | D (dB) | g = 10^(D/20)      | `gs`                     |
| Dry/Wet | M (%)  | m = M/100          | `ms`                     |
| Output  | O (dB) | g_out = 10^(O/20)  | `g_out_s`                |

A new knob that reaches the audio gets its own `Smoother` and a row in this table.

### Audio path (per channel)

```text
                            x[n]  (per channel)
                            │
                            ├───────────────────────┐ dry
                            ▼                       │
               ┌─────────────────────────┐          │
       gs ────►│          Drive          │          │
               │       v = gs · x        │          │
               └────────────┬────────────┘          │
                            │ v[n]                  │
                            ▼                       │
               ┌─────────────────────────┐          │
               │          Curve          │          │
               │       y = tanh(v)       │          │
               └────────────┬────────────┘          │
                            │ y[n]  (wet)           │
                            ▼                       │
               ┌─────────────────────────┐          │
       ms ────►│           Mix           │◄─────────┘
               │   w = (1-ms)·x + ms·y   │
               └────────────┬────────────┘
                            │ w[n]
                            ▼
               ┌─────────────────────────┐
  g_out_s ────►│         Output          │
               │   y_out = g_out_s · w   │
               └────────────┬────────────┘
                            │ y_out[n]  (per channel)
                            ▼
```

All four audio blocks are memoryless: Drive, Mix and Output are products of signals,
Curve is a static nonlinearity. The dry signal is the input x[n] before Drive.

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
│   │   ├── mix.rs           # memoryless dry/wet crossfade
│   │   ├── output.rs        # memoryless final attenuation
│   │   ├── smoother.rs      # one-pole LPF, has state (one instance per knob)
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
| `output`       | Output       | −36 … 0   | 0         | dB   | `−6.0 dB`   | wired to DSP            |
| `dry_wet`      | Dry/Wet      | 0 … 100   | 100       | %    | `75 %`      | wired to DSP            |
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
- The GUI changes every parameter; Drive, Dry/Wet and Output reach the audio, each through
  its own smoother (see Signal flow › Control path).
- Dry/Wet is stored in % (what the host shows) and converted to [0, 1] before its smoother.

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
| `activate()`   | main   | Store fs, compute `a = e^(-1/(τ·fs))` for every smoother, allocate everything |
| `reset()`      | audio  | Snap every smoother to its current converted value p (no ramp)    |
| `process()`    | audio  | Per frame: advance every smoother; per channel: audio path        |

## State

- Smoothers: one instance per knob (Drive, Dry/Wet, Output), each shared by both channels
  (a knob is a single parameter), persisting across buffers.
- Drive, Curve, Mix and Output: stateless (pure functions), applied to each channel.
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
- Harmonic levels (dBc) must match the reference values in the notes (Dry/Wet at 100 %,
  Output at 0 dB).
- Dry/Wet at 0 %: no harmonics, the output is the input scaled by the Output gain.
- Moving any knob must not produce clicks or zipper noise.

## Known limitations

- No oversampling: harmonics above fs/2 alias. Audible with high Drive and high f0.
- No auto gain: loudness rises with Drive until the curve limits it; Output compensates
  only by hand.
- Channels processed independently (no stereo linking).

## Roadmap

1. Oversampling around the curve (report latency to host).
2. Auto gain compensation.
3. Asymmetric curve (bias) ⇒ even harmonics.
4. Pre/de-emphasis filters around the curve (from eq-rs).