//! Sprite-sheet metadata (`assets/gui/sprites.json`) and frame selection.
//!
//! Sheets are row-major grids of equally sized frames. All sizes in the manifest are in asset
//! pixels, which are `scale` times the editor's logical pixels.

use nih_plug_egui::egui::{pos2, vec2, Rect, Vec2};
use serde::Deserialize;
use std::collections::BTreeMap;

/// The manifest, embedded at compile time.
pub const MANIFEST_JSON: &str = include_str!("../../assets/gui/sprites.json");

#[derive(Debug, Clone, Deserialize)]
pub struct SpriteManifest {
    /// Asset pixels per logical pixel (2 for the 2x assets).
    pub scale: f32,
    pub sprites: BTreeMap<String, SpriteSheet>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpriteSheet {
    pub file: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub frame_count: u32,
    pub columns: u32,
    pub rows: u32,
    /// Pointer angle of the first frame. 0° = up, positive = clockwise.
    /// Checked against the artwork and the panel scale by the tests; not needed to draw.
    #[cfg_attr(not(test), allow(dead_code))]
    pub angle_min_deg: f32,
    /// Pointer angle of the last frame.
    #[cfg_attr(not(test), allow(dead_code))]
    pub angle_max_deg: f32,
    /// Parameter IDs drawn with this sheet (checked against the layout by the tests).
    #[serde(default)]
    #[cfg_attr(not(test), allow(dead_code))]
    pub used_by: Vec<String>,
}

impl SpriteManifest {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The embedded manifest. It is validated by the tests, so failing here is a build defect.
    pub fn embedded() -> Self {
        Self::parse(MANIFEST_JSON).expect("embedded sprites.json is valid (checked by tests)")
    }

    pub fn sheet(&self, key: &str) -> &SpriteSheet {
        self.sprites
            .get(key)
            .unwrap_or_else(|| panic!("sprite '{key}' missing from sprites.json"))
    }
}

impl SpriteSheet {
    /// Size of one frame on screen, in logical pixels.
    pub fn display_size(&self, scale: f32) -> Vec2 {
        vec2(self.frame_width as f32, self.frame_height as f32) / scale
    }

    /// Pointer angle of frame `k`, in degrees (0° = up, clockwise positive).
    #[cfg(test)]
    pub fn frame_angle_deg(&self, k: u32) -> f32 {
        frame_angle_deg(k, self.frame_count, self.angle_min_deg, self.angle_max_deg)
    }

    /// Texture coordinates of frame `k`.
    pub fn uv_rect(&self, k: u32) -> Rect {
        uv_rect(k, self.columns, self.rows)
    }
}

/// Frame for a normalized value: `k = round(v · (frame_count − 1))`. `v` is clamped to [0, 1].
pub fn frame_index(v: f32, frame_count: u32) -> u32 {
    let last = frame_count.saturating_sub(1);
    let v = if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
    ((v * last as f32).round() as u32).min(last)
}

/// UV rectangle of frame `k` in a row-major grid of `columns × rows` frames.
pub fn uv_rect(k: u32, columns: u32, rows: u32) -> Rect {
    let (col, row) = (k % columns, k / columns);
    let (du, dv) = (1.0 / columns as f32, 1.0 / rows as f32);
    Rect::from_min_size(pos2(col as f32 * du, row as f32 * dv), vec2(du, dv))
}

/// Angle of frame `k` when `frame_count` frames span `[angle_min, angle_max]` evenly.
#[cfg(test)]
pub fn frame_angle_deg(k: u32, frame_count: u32, angle_min: f32, angle_max: f32) -> f32 {
    if frame_count <= 1 {
        return angle_min;
    }
    angle_min + (angle_max - angle_min) * k as f32 / (frame_count - 1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> SpriteManifest {
        SpriteManifest::parse(MANIFEST_JSON).expect("sprites.json parses")
    }

    #[test]
    fn frame_index_endpoints_and_rounding() {
        assert_eq!(frame_index(0.0, 128), 0);
        assert_eq!(frame_index(1.0, 128), 127);
        // 0.5 · 127 = 63.5 → rounds away from zero.
        assert_eq!(frame_index(0.5, 128), 64);
        assert_eq!(frame_index(0.25, 128), 32); // 31.75
        assert_eq!(frame_index(1.0 / 3.0, 4), 1);
        assert_eq!(frame_index(2.0 / 3.0, 4), 2);
    }

    #[test]
    fn frame_index_clamps_out_of_range_values() {
        assert_eq!(frame_index(-0.5, 128), 0);
        assert_eq!(frame_index(1.5, 128), 127);
        assert_eq!(frame_index(f32::NAN, 128), 0);
        assert_eq!(frame_index(0.7, 1), 0);
    }

    #[test]
    fn uv_rect_is_row_major() {
        let first = uv_rect(0, 16, 8);
        assert_eq!(first.min, pos2(0.0, 0.0));
        assert_eq!(first.max, pos2(1.0 / 16.0, 1.0 / 8.0));

        // Frame 17 = row 1, column 1.
        let r = uv_rect(17, 16, 8);
        assert_eq!(r.min, pos2(1.0 / 16.0, 1.0 / 8.0));
        assert_eq!(r.max, pos2(2.0 / 16.0, 2.0 / 8.0));

        let last = uv_rect(127, 16, 8);
        assert_eq!(last.min, pos2(15.0 / 16.0, 7.0 / 8.0));
        assert_eq!(last.max, pos2(1.0, 1.0));

        let single = uv_rect(0, 1, 1);
        assert_eq!(single, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
    }

    #[test]
    fn frame_angles_span_the_scale() {
        assert_eq!(frame_angle_deg(0, 128, -135.0, 135.0), -135.0);
        assert_eq!(frame_angle_deg(127, 128, -135.0, 135.0), 135.0);
        assert_eq!(frame_angle_deg(2, 4, -135.0, 135.0), 45.0);
        // A single-frame sheet (screws) sits at its fixed angle.
        assert_eq!(frame_angle_deg(0, 1, 35.0, 35.0), 35.0);
    }

    #[test]
    fn manifest_parses_and_grids_are_consistent() {
        let m = manifest();
        assert_eq!(m.scale, 2.0);
        for key in [
            "drive",
            "knob_small",
            "curve",
            "screw_tl",
            "screw_tr",
            "screw_bl",
            "screw_br",
        ] {
            let s = m.sheet(key);
            assert!(s.frame_count >= 1, "{key}");
            assert!(s.frame_count <= s.columns * s.rows, "{key}");
        }
    }

    #[test]
    fn knob_sheets_point_at_the_scale_ends() {
        let m = manifest();
        for key in ["drive", "knob_small", "curve"] {
            let s = m.sheet(key);
            assert_eq!(s.frame_angle_deg(0), -135.0, "{key} min");
            assert_eq!(s.frame_angle_deg(s.frame_count - 1), 135.0, "{key} max");
        }
        // The curve selector's four detents.
        let c = m.sheet("curve");
        let angles: Vec<f32> = (0..4).map(|k| c.frame_angle_deg(k)).collect();
        assert_eq!(angles, vec![-135.0, -45.0, 45.0, 135.0]);
    }

    #[test]
    fn display_size_is_half_the_asset_size() {
        let m = manifest();
        assert_eq!(m.sheet("drive").display_size(m.scale), vec2(128.0, 128.0));
        assert_eq!(m.sheet("screw_tl").display_size(m.scale), vec2(16.0, 16.0));
    }
}
