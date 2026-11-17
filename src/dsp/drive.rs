//! Drive stage. Memoryless: the product of two signals, v[n] = gs[n] · x[n].

/// Applies the (smoothed) linear gain `gs` to the input sample `x`.
#[inline]
pub fn drive(gs: f32, x: f32) -> f32 {
    gs * x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplies_gain_and_input() {
        assert_eq!(drive(2.0, 0.25), 0.5);
        assert_eq!(drive(1.0, -0.7), -0.7);
        assert_eq!(drive(0.0, 0.9), 0.0);
    }
}
