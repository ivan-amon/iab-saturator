//! Final output attenuation. Memoryless: the product of two signals, y_out[n] = gs[n] · y[n].

// Applies the dry/wet to the signal
// - x[n]: dry
// - y[n]: wet
#[inline]
pub fn mix(m: f32, x: f32, y: f32) -> f32 {
    (1.0 - m) * x + m * y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_dry_wet() {
        assert_eq!(mix(0.0, 0.5, 0.9), 0.5);
        assert_eq!(mix(1.0, 0.5, 0.9), 0.9);
        assert_eq!(mix(0.25, 0.5, 0.9), 0.6);
    }
}
