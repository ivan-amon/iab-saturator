//! dB → linear gain conversion. Memoryless.

/// Converts a level in dB to a linear gain: g = 10^(D/20).
#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    #[test]
    fn zero_db_is_unity() {
        assert!((db_to_gain(0.0) - 1.0).abs() < EPS);
    }

    #[test]
    fn twenty_db_is_ten() {
        assert!((db_to_gain(20.0) - 10.0).abs() < 10.0 * EPS);
    }

    #[test]
    fn six_db_is_almost_two() {
        // 10^(6/20) = 1.99526...
        assert!((db_to_gain(6.0) - 1.995_262_3).abs() < EPS);
    }

    #[test]
    fn negative_thirty_six_db_is_0_0158() {
        assert!((db_to_gain(-36.0) - 0.015_848_9).abs() < EPS);
    }
}
