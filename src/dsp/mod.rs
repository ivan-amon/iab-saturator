//! Pure DSP blocks. No framework dependencies: this module only maps samples to samples.
//!
//! One module per block of the block diagram in ARCHITECTURE.md:
//! - `db`: dB → linear gain (memoryless)
//! - `smoother`: one-pole LPF on the linear gain (has state)
//! - `drive`: gain stage (memoryless)
//! - `curve`: tanh waveshaper (memoryless)

pub mod curve;
pub mod db;
pub mod drive;
pub mod output;
pub mod smoother;

#[cfg(test)]
mod harmonics_test;
