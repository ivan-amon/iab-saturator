//! Plugin parameters. IDs (`#[id]` and enum variant `#[id]`s) are stored in saved projects and
//! automation: never change them.

use nice_plug::prelude::*;
use std::sync::Arc;

/// U+2212 MINUS SIGN, used in readouts instead of the ASCII hyphen.
const MINUS_SIGN: char = '\u{2212}';

#[derive(Params)]
pub struct IabSaturatorParams {
    /// Drive in dB. Smoothing is done by our own `Smoother` on the linear gain, so nice-plug's
    /// built-in smoother is left disabled (the `FloatParam::new` default).
    #[id = "drive"]
    pub drive: FloatParam,

    /// Output level in dB. Declared, not wired to the DSP yet (no smoother until it is).
    #[id = "output"]
    pub output: FloatParam,

    /// Dry/wet mix in %. Declared, not wired to the DSP yet (no smoother until it is).
    #[id = "dry_wet"]
    pub dry_wet: FloatParam,

    /// Waveshaper curve. Declared, not wired to the DSP yet.
    #[id = "curve"]
    pub curve: EnumParam<CurveType>,

    /// Oversampling factor. Declared, not wired to the DSP yet.
    #[id = "oversampling"]
    pub oversampling: EnumParam<Oversampling>,

    /// Automatic gain compensation. Declared, not wired to the DSP yet.
    #[id = "auto_gain"]
    pub auto_gain: BoolParam,
}

/// Curve selector positions, in panel order (the order of `selector.positions` in layout.json).
/// SOFT CLIP keeps the ID `tanh`: it is the tanh curve of the DSP, so older projects still load.
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    #[id = "tanh"]
    #[name = "SOFT CLIP"]
    SoftClip,
    #[id = "hard_clip"]
    #[name = "HARD CLIP"]
    HardClip,
    #[id = "warm"]
    #[name = "WARM"]
    Warm,
    #[id = "tube"]
    #[name = "TUBE"]
    Tube,
    #[id = "fold"]
    #[name = "FOLD"]
    Fold,
    #[id = "rectify"]
    #[name = "RECTIFY"]
    Rectify,
    #[id = "bit_crush"]
    #[name = "BIT CRUSH"]
    BitCrush,
}

/// Oversampling factors, in panel order (`bottom_bar.oversampling.segments` in layout.json).
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Oversampling {
    #[id = "off"]
    #[name = "OFF"]
    Off,
    #[id = "x2"]
    #[name = "2×"]
    X2,
    #[id = "x4"]
    #[name = "4×"]
    X4,
    #[id = "x8"]
    #[name = "8×"]
    X8,
}

impl Default for IabSaturatorParams {
    fn default() -> Self {
        Self {
            drive: FloatParam::new(
                "Drive",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 36.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(Arc::new(format_db_signed))
            .with_string_to_value(Arc::new(parse_db)),

            output: FloatParam::new(
                "Output",
                0.0,
                FloatRange::Linear {
                    min: -36.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(Arc::new(format_db_minus_sign))
            .with_string_to_value(Arc::new(parse_db)),

            dry_wet: FloatParam::new(
                "Dry/Wet",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            curve: EnumParam::new("Curve", CurveType::SoftClip),
            oversampling: EnumParam::new("Oversampling", Oversampling::Off),
            auto_gain: BoolParam::new("Auto Gain", false),
        }
    }
}

/// Drive readout: one decimal, always signed (`+15.0`, `+0.0`).
pub fn format_db_signed(value: f32) -> String {
    format!("{:+.1}", value)
}

/// Output readout: one decimal, negative values with U+2212 (`−6.0`), never `−0.0`.
pub fn format_db_minus_sign(value: f32) -> String {
    let text = format!("{:.1}", value.abs());
    // Decide the sign from the rounded text, so -0.04 reads "0.0" and not "−0.0".
    if value < 0.0 && text.bytes().any(|b| b.is_ascii_digit() && b != b'0') {
        format!("{MINUS_SIGN}{text}")
    } else {
        text
    }
}

/// Parses what a user types in the host: optional sign (`+`, `-` or U+2212), optional ` dB`.
pub fn parse_db(text: &str) -> Option<f32> {
    let text = text.trim();
    let text = text.strip_suffix("dB").unwrap_or(text).trim_end();
    text.replace(MINUS_SIGN, "-").parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_readout_is_signed() {
        assert_eq!(format_db_signed(15.0), "+15.0");
        assert_eq!(format_db_signed(0.0), "+0.0");
        assert_eq!(format_db_signed(36.0), "+36.0");
    }

    #[test]
    fn output_readout_uses_the_minus_sign() {
        assert_eq!(format_db_minus_sign(-6.0), "\u{2212}6.0");
        assert_eq!(format_db_minus_sign(-36.0), "\u{2212}36.0");
        assert_eq!(format_db_minus_sign(0.0), "0.0");
    }

    #[test]
    fn output_readout_never_shows_negative_zero() {
        assert_eq!(format_db_minus_sign(-0.0), "0.0");
        assert_eq!(format_db_minus_sign(-0.04), "0.0");
    }

    #[test]
    fn parse_accepts_both_minus_signs_plus_and_unit() {
        assert_eq!(parse_db("\u{2212}6.0 dB"), Some(-6.0));
        assert_eq!(parse_db("-6"), Some(-6.0));
        assert_eq!(parse_db("+15.0 dB"), Some(15.0));
        assert_eq!(parse_db(" 12dB "), Some(12.0));
        assert_eq!(parse_db("abc"), None);
    }

    #[test]
    fn readouts_include_the_unit() {
        let params = IabSaturatorParams::default();
        assert_eq!(
            params.drive.normalized_value_to_string(15.0 / 36.0, true),
            "+15.0 dB"
        );
        assert_eq!(
            params.output.normalized_value_to_string(30.0 / 36.0, true),
            "\u{2212}6.0 dB"
        );
        assert_eq!(
            params.dry_wet.normalized_value_to_string(0.75, true),
            "75 %"
        );
    }

    #[test]
    fn soft_clip_keeps_the_tanh_id_and_is_the_default() {
        assert_eq!(CurveType::ids().map(|ids| ids[0]), Some("tanh"));
        assert_eq!(
            IabSaturatorParams::default().curve.value(),
            CurveType::SoftClip
        );
    }

    #[test]
    fn enum_display_names_are_the_panel_texts() {
        assert_eq!(
            CurveType::variants(),
            [
                "SOFT CLIP",
                "HARD CLIP",
                "WARM",
                "TUBE",
                "FOLD",
                "RECTIFY",
                "BIT CRUSH"
            ]
        );
        assert_eq!(Oversampling::variants(), ["OFF", "2×", "4×", "8×"]);
    }

    #[test]
    fn new_params_default_to_off() {
        let params = IabSaturatorParams::default();
        assert_eq!(params.oversampling.value(), Oversampling::Off);
        assert!(!params.auto_gain.value());
    }
}
