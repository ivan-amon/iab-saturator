//! Harmonic test for drive + curve.
//!
//! A sine with an integer number of periods `K0` in an `N`-sample window (coherent sampling) puts
//! the fundamental exactly on bin K0 and its harmonics on bins m·K0, so no window is needed.
//! With K0 odd and N even, odd harmonics above fs/2 also alias onto odd bins only, so the
//! even-harmonic bins (2·K0, 4·K0, ...) stay clean and can only be filled by an even component.

use super::{curve::curve, db::db_to_gain, drive::drive};
use std::f64::consts::PI;

const N: usize = 4096;
/// Periods in the window. Odd, see module docs.
const K0: usize = 31;

/// Magnitude of DFT bin `k` (naive single-bin DFT, computed in f64).
fn dft_mag(x: &[f32], k: usize) -> f64 {
    let n_len = x.len() as f64;
    let (mut re, mut im) = (0.0_f64, 0.0_f64);
    for (n, &xn) in x.iter().enumerate() {
        let phase = 2.0 * PI * (k * n) as f64 / n_len;
        re += xn as f64 * phase.cos();
        im -= xn as f64 * phase.sin();
    }
    re.hypot(im)
}

fn dbc(harmonic: f64, fundamental: f64) -> f64 {
    20.0 * (harmonic / fundamental).log10()
}

/// Sine of amplitude `amp` through drive (constant gain from `drive_db`) and curve.
fn saturate_sine(amp: f64, drive_db: f32) -> Vec<f32> {
    let g = db_to_gain(drive_db);
    (0..N)
        .map(|n| {
            let x = (amp * (2.0 * PI * (K0 * n) as f64 / N as f64).sin()) as f32;
            curve(drive(g, x))
        })
        .collect()
}

#[test]
fn only_odd_harmonics_at_high_drive() {
    let y = saturate_sine(1.0, 36.0);
    let h1 = dft_mag(&y, K0);

    for m in [2, 4, 6] {
        let level = dbc(dft_mag(&y, m * K0), h1);
        assert!(
            level < -100.0,
            "harmonic {m} at {level:.1} dBc, expected ≈ 0"
        );
    }

    // Close to a square wave: the 3rd harmonic tends to 1/3 of the fundamental (-9.5 dBc).
    let h3 = dbc(dft_mag(&y, 3 * K0), h1);
    assert!(h3 > -20.0, "3rd harmonic only {h3:.1} dBc at 36 dB drive");
}

#[test]
fn nearly_transparent_at_small_signal() {
    // |g·x| = 1e-3 ≪ 1: y ≈ g·x, so the 3rd harmonic is tiny.
    let y = saturate_sine(1e-3, 0.0);
    let h1 = dft_mag(&y, K0);
    let h3 = dbc(dft_mag(&y, 3 * K0), h1);
    assert!(
        h3 < -100.0,
        "3rd harmonic at {h3:.1} dBc for a small signal"
    );
}
