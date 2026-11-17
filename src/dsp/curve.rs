//! Waveshaper. Memoryless static nonlinearity: y = tanh(v).

/// The saturation curve.
#[inline]
pub fn curve(v: f32) -> f32 {
    v.tanh()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inputs from -100 to +100 in steps of 0.01, covering both the linear region and deep
    /// saturation (36 dB drive on a full-scale input gives v ≈ 63).
    fn sweep() -> impl Iterator<Item = f32> {
        (-10_000..=10_000).map(|i| i as f32 * 0.01)
    }

    #[test]
    fn is_odd() {
        for v in sweep() {
            assert_eq!(curve(-v), -curve(v), "f(-v) != -f(v) at v = {v}");
        }
    }

    #[test]
    fn is_transparent_for_small_signals() {
        // tanh(v) = v - v^3/3 + ..., so the relative error at |v| <= 1e-3 is below 3.4e-7.
        for i in -1000..=1000 {
            let v = i as f32 * 1e-6;
            if v == 0.0 {
                assert_eq!(curve(v), 0.0);
                continue;
            }
            let rel_err = ((curve(v) - v) / v).abs();
            assert!(
                rel_err < 1e-5,
                "f(v) not ≈ v at v = {v}: rel_err = {rel_err}"
            );
        }
    }

    #[test]
    fn is_bounded() {
        // In f32, tanh(v) rounds to exactly ±1.0 for |v| > ~9, so the bound is |f(v)| <= 1.
        for v in sweep() {
            assert!(curve(v).abs() <= 1.0, "|f(v)| > 1 at v = {v}");
        }
        assert!(curve(f32::MAX).abs() <= 1.0);
        assert!(curve(f32::MIN).abs() <= 1.0);
    }
}
