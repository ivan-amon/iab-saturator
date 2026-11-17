//! One-pole low-pass smoother. Has state.
//!
//!   gs[n] = (1 - a) · g[n] + a · gs[n-1]
//!   a     = e^(-1 / (τ · fs))

#[derive(Debug, Clone, Copy, Default)]
pub struct Smoother {
    /// Pole of the filter, a = e^(-1/(τ·fs)).
    a: f32,
    /// Previous output, gs[n-1].
    state: f32,
}

impl Smoother {
    /// Computes the coefficient for sampling rate `fs` (Hz) and time constant `tau_ms` (ms).
    pub fn set_sample_rate(&mut self, fs: f32, tau_ms: f32) {
        let tau = tau_ms / 1000.0; // ms → s
        self.a = (-1.0 / (tau * fs)).exp();
    }

    /// Snaps the state to `value`, so the next output starts there with no ramp.
    pub fn reset(&mut self, value: f32) {
        self.state = value;
    }

    /// Advances one sample towards `target` and returns the smoothed value gs[n].
    #[inline]
    pub fn next(&mut self, target: f32) -> f32 {
        self.state = (1.0 - self.a) * target + self.a * self.state;
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::db::db_to_gain;

    const FS: f32 = 48_000.0;
    const TAU_MS: f32 = 10.0;
    /// τ · fs = 0.010 s · 48000 Hz = 480 samples.
    const TAU_SAMPLES: usize = 480;

    fn smoother() -> Smoother {
        let mut s = Smoother::default();
        s.set_sample_rate(FS, TAU_MS);
        s
    }

    #[test]
    fn coefficient_matches_definition() {
        let s = smoother();
        let expected = (-1.0_f64 / TAU_SAMPLES as f64).exp() as f32;
        assert!((s.a - expected).abs() < 1e-7);
    }

    #[test]
    fn dc_gain_is_one() {
        // In f32 the state stops moving once the update (1-a)·(g - gs) is below one ulp of gs,
        // i.e. it settles within about ulp(g)/(1-a) of the target. With 1-a ≈ 2.1e-3 that is a
        // relative error of ~2.4e-4 at most (≈ 0.002 dB), so that is the tolerance used here.
        for target in [1.0, 3.5, db_to_gain(36.0)] {
            let mut s = smoother();
            s.reset(0.0);
            let mut y = 0.0;
            for _ in 0..20 * TAU_SAMPLES {
                y = s.next(target);
            }
            let tol = f32::EPSILON * target / (1.0 - s.a);
            assert!(
                (y - target).abs() <= tol,
                "did not converge to {target}: {y} (tol {tol})"
            );
        }
    }

    #[test]
    fn step_reaches_63_percent_after_tau() {
        let mut s = smoother();
        s.reset(0.0);
        let mut y = 0.0;
        for _ in 0..TAU_SAMPLES {
            y = s.next(1.0);
        }
        let expected = 1.0 - (-1.0_f32).exp(); // 0.632...
        assert!(
            (y - expected).abs() < 1e-3,
            "y = {y}, expected ≈ {expected}"
        );
    }

    #[test]
    fn step_response_is_monotonic_and_below_target() {
        let mut s = smoother();
        s.reset(0.0);
        let mut prev = 0.0;
        for _ in 0..5 * TAU_SAMPLES {
            let y = s.next(1.0);
            assert!(y > prev && y <= 1.0);
            prev = y;
        }
    }

    #[test]
    fn reset_snaps_without_ramp() {
        let mut s = smoother();
        s.reset(0.0);
        for _ in 0..10 {
            s.next(1.0);
        }
        s.reset(5.0);
        for _ in 0..10 {
            let y = s.next(5.0);
            assert!((y - 5.0).abs() < 1e-6, "ramp after reset: {y}");
        }
    }
}
