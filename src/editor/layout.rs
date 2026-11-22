//! `assets/gui/layout.json`: every position, size, colour and angle the editor uses.
//! Only the fields the editor needs are deserialized; the Blender-only blocks are ignored.
//! Units: px at 1x (egui points), origin top-left, angles in degrees (0 = 12 o'clock, clockwise).

use egui::Color32;
use serde::Deserialize;
use std::collections::BTreeMap;

const LAYOUT_JSON: &str = include_str!("../../assets/gui/layout.json");

#[derive(Debug, Deserialize)]
pub struct Layout {
    pub meta: Meta,
    pub colors: BTreeMap<String, String>,
    pub fonts: Fonts,
    pub knobs: Vec<Knob>,
    pub selector: Selector,
    pub meters: Vec<Meter>,
    pub meter_scale: MeterScale,
    pub bottom_bar: BottomBar,
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    pub panel: Size,
    pub render_scale: f32,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub struct Size {
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Centre {
    pub cx: f32,
    pub cy: f32,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct BoxXywh {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct BoxCentred {
    pub cx: f32,
    pub cy: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Deserialize)]
pub struct Fonts {
    pub cap_height_em: f32,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Sweep {
    pub from: f32,
    pub to: f32,
}

/// A value pill. `color` is the pill (baked into bg.png); the editor draws only the text.
#[derive(Debug, Deserialize)]
pub struct Readout {
    pub cx: f32,
    pub cy: f32,
    pub h: f32,
    pub text_color: String,
}

#[derive(Debug, Deserialize)]
pub struct Knob {
    /// Asset name: `knob_<id>_body.png`, `knob_<id>_pointer.png`.
    pub id: String,
    /// Parameter ID it controls.
    pub param: String,
    pub cx: f32,
    pub cy: f32,
    pub body_d: f32,
    #[cfg_attr(not(test), allow(dead_code))] // checked by the tests
    pub sprite: Size,
    pub sweep: Sweep,
    pub readout: Readout,
}

#[derive(Debug, Deserialize)]
pub struct Selector {
    #[cfg_attr(not(test), allow(dead_code))] // checked by the tests
    pub param: String,
    pub cx: f32,
    pub cy: f32,
    pub body_d: f32,
    #[cfg_attr(not(test), allow(dead_code))] // checked by the tests
    pub sprite: Size,
    pub positions: Vec<SelectorPosition>,
    pub position_leds: PositionLeds,
    pub readout: Readout,
}

#[derive(Debug, Deserialize)]
pub struct SelectorPosition {
    pub index: usize,
    #[cfg_attr(not(test), allow(dead_code))] // checked by the tests
    pub value: String,
    pub led: Centre,
    pub label_sprite: BoxCentred,
}

#[derive(Debug, Deserialize)]
pub struct PositionLeds {
    pub sprite: Size,
}

#[derive(Debug, Deserialize)]
pub struct Meter {
    /// `in` or `out`: asset name `meter_glass_<id>.png`.
    pub id: String,
    pub housing: BoxXywh,
    pub face: BoxXywh,
    pub pivot: Point,
}

#[derive(Debug, Deserialize)]
pub struct MeterScale {
    pub rest_angle: f32,
}

#[derive(Debug, Deserialize)]
pub struct BottomBar {
    pub oversampling: SegmentedControl,
    pub toggles: Vec<Toggle>,
    #[cfg_attr(not(test), allow(dead_code))] // checked by the tests
    pub led: Led,
}

#[derive(Debug, Deserialize)]
pub struct SegmentedControl {
    #[cfg_attr(not(test), allow(dead_code))] // checked by the tests
    pub param: String,
    #[serde(rename = "box")]
    pub bounds: BoxXywh,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Deserialize)]
pub struct Segment {
    pub index: usize,
    #[cfg_attr(not(test), allow(dead_code))] // checked by the tests
    pub text: String,
    pub x: f32,
    pub w: f32,
}

#[derive(Debug, Deserialize)]
pub struct Toggle {
    /// Asset name: `led_<id>_on.png`.
    pub id: String,
    pub param: String,
    pub led: Centre,
    pub hit: BoxXywh,
}

#[derive(Debug, Deserialize)]
pub struct Led {
    #[cfg_attr(not(test), allow(dead_code))] // checked by the tests
    pub sprite: Size,
}

impl Layout {
    /// Parses the embedded layout.json. The tests below guarantee it parses.
    pub fn load() -> Result<Self, serde_json::Error> {
        serde_json::from_str(LAYOUT_JSON)
    }

    /// A colour of the `colors` table, `#rrggbb`. Unknown names and malformed values are
    /// rejected by the tests; at run time they fall back to magenta so they are visible.
    pub fn color(&self, name: &str) -> Color32 {
        self.colors
            .get(name)
            .and_then(|hex| parse_hex_color(hex))
            .unwrap_or(Color32::from_rgb(255, 0, 255))
    }
}

/// `#rrggbb` → opaque colour.
pub fn parse_hex_color(hex: &str) -> Option<Color32> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(digits.get(i..i + 2)?, 16).ok();
    Some(Color32::from_rgb(channel(0)?, channel(2)?, channel(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{CurveType, IabSaturatorParams, Oversampling};
    use nice_plug::prelude::{Enum, Params};

    fn layout() -> Layout {
        Layout::load().expect("layout.json must parse")
    }

    #[test]
    fn parses() {
        layout();
    }

    #[test]
    fn hex_colours_parse() {
        assert_eq!(
            parse_hex_color("#b73025"),
            Some(Color32::from_rgb(0xb7, 0x30, 0x25))
        );
        assert_eq!(parse_hex_color("b73025"), None);
        assert_eq!(parse_hex_color("#b7302"), None);
        assert_eq!(parse_hex_color("#zz3025"), None);
    }

    #[test]
    fn every_colour_used_by_the_editor_exists() {
        let l = layout();
        let mut used: Vec<&str> = l
            .knobs
            .iter()
            .map(|k| k.readout.text_color.as_str())
            .collect();
        used.push(&l.selector.readout.text_color);
        for name in used {
            let hex = l
                .colors
                .get(name)
                .unwrap_or_else(|| panic!("missing colour {name}"));
            assert!(parse_hex_color(hex).is_some(), "bad colour {name} = {hex}");
        }
    }

    #[test]
    fn every_bound_param_exists() {
        let l = layout();
        let params = IabSaturatorParams::default();
        let ids: Vec<String> = params
            .param_map()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        let mut bound: Vec<&str> = l.knobs.iter().map(|k| k.param.as_str()).collect();
        bound.push(&l.selector.param);
        bound.push(&l.bottom_bar.oversampling.param);
        bound.extend(l.bottom_bar.toggles.iter().map(|t| t.param.as_str()));
        for id in bound {
            assert!(
                ids.iter().any(|p| p == id),
                "layout binds unknown param {id}"
            );
        }
    }

    #[test]
    fn selector_positions_match_the_curve_enum() {
        let l = layout();
        let values: Vec<&str> = l
            .selector
            .positions
            .iter()
            .map(|p| p.value.as_str())
            .collect();
        assert_eq!(values, CurveType::variants());
        for (i, p) in l.selector.positions.iter().enumerate() {
            assert_eq!(p.index, i);
        }
    }

    #[test]
    fn oversampling_segments_match_the_enum() {
        let l = layout();
        let texts: Vec<&str> = l
            .bottom_bar
            .oversampling
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(texts, Oversampling::variants());
        for (i, s) in l.bottom_bar.oversampling.segments.iter().enumerate() {
            assert_eq!(s.index, i);
        }
    }

    #[test]
    fn knob_sweeps_match_the_panel_scale() {
        // CLAUDE.md: −135° at minimum, +135° at maximum.
        for k in &layout().knobs {
            assert_eq!((k.sweep.from, k.sweep.to), (-135.0, 135.0), "knob {}", k.id);
        }
    }
}
