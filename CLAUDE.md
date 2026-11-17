# IAB-Saturator

Audio saturation plugin (VST3 + CLAP). Learning project: the DSP math is mine,
Claude helps with plugin plumbing, tooling and review.

Read ARCHITECTURE.md before touching code.

## Stack
- Rust, nih-plug
- Target: Windows 11, Ableton Live 12

## Commands
- Build bundles: `cargo xtask bundle iab_saturator --release`
- Tests: `cargo test`
- Install: copy the .vst3 to `C:\Program Files\Common Files\VST3`
  and the .clap to `C:\Program Files\Common Files\CLAP`

## Rules
- Do NOT write formulas or DSP modules without asking first.
- Real-time safety in `process()`: no allocations, no locks, no I/O, no logging.
- One module per block of the block diagram. Memoryless blocks are pure
  functions; blocks with state are structs with one instance per channel.
- Parameters are smoothed; never apply a raw parameter jump to the audio.
- TDD: tests before logic.
- Code, comments and names in English.

## Notation (UPM, not the US convention)
- `fs` sampling rate, `ts` sampling period
- ω: analog angular frequency, Ω: discrete angular frequency (Ω = ω·Ts)
- Drive in dB, linear gain g = 10^(D/20)

## Current scope
- Only the Drive knob + tanh curve. No oversampling, no output gain yet.

## Verification
- Sine in → Spectrum after the plugin. Only odd harmonics must appear.
- Compare harmonic levels (dBc) against the values predicted in my notes.