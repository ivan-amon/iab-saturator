//! Final output attenuation. Memoryless: the product of two signals, y_out[n] = gs[n] · y[n].

// Applies the linear gain `g` to the input sample `y`. [-36, 0] in dB
#[inline]
pub fn output(g: f32, y: f32) -> f32 {
    g * y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplies_gain_and_output() {
        assert_eq!(output(0.0, 0.9), 0.0);
        assert_eq!(output(0.05, 2.0), 0.1);
    }
}
