//! Pixel checks of the artwork against the layout (test-only):
//! - the pointer drawn in every sprite frame points where `sprites.json` says it does;
//! - the panel's scale ticks sit at −135° / +135° around each control's layout center, so the
//!   minimum frame points at the first tick and the maximum frame at the last.
//!
//! Angles: 0° = up, positive = clockwise (the `sprites.json` convention).

use super::assets::{decode_png, DecodedAssets, RgbaImage, PANEL_PNG};
use super::layout::{ControlLayout, CONTROLS, DRIVE};
use super::sprites::{SpriteManifest, SpriteSheet};

const POINTER_TOLERANCE_DEG: f32 = 2.5;
const TICK_TOLERANCE_DEG: f32 = 2.0;

fn luminance(p: [u8; 4]) -> f32 {
    0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32
}

/// Angle of the vector (dx, dy) in screen coordinates (y down).
fn screen_angle_deg(dx: f32, dy: f32) -> f32 {
    dx.atan2(-dy).to_degrees()
}

/// Smallest signed difference a − b, in degrees.
fn angle_diff(a: f32, b: f32) -> f32 {
    (a - b + 540.0).rem_euclid(360.0) - 180.0
}

/// Direction of the light pointer stripe in frame `k`: centroid of the near-white pixels
/// relative to the frame center.
fn pointer_angle(image: &RgbaImage, sheet: &SpriteSheet, k: u32) -> f32 {
    let (fw, fh) = (sheet.frame_width, sheet.frame_height);
    let (x0, y0) = ((k % sheet.columns) * fw, (k / sheet.columns) * fh);
    let (cx, cy) = (fw as f32 / 2.0, fh as f32 / 2.0);
    let (mut sx, mut sy, mut n) = (0.0f32, 0.0f32, 0u32);
    for y in 0..fh {
        for x in 0..fw {
            let p = image.pixel(x0 + x, y0 + y);
            if p[3] >= 200 && p[0].min(p[1]).min(p[2]) >= 170 {
                sx += x as f32 + 0.5 - cx;
                sy += y as f32 + 0.5 - cy;
                n += 1;
            }
        }
    }
    assert!(n > 0, "no pointer pixels found in frame {k}");
    screen_angle_deg(sx, sy)
}

#[test]
fn every_frame_points_at_its_manifest_angle() {
    let assets = DecodedAssets::decode();
    for key in ["drive", "knob_small", "curve"] {
        let sheet = assets.manifest.sheet(key);
        let image = &assets.sheets[key];
        for k in 0..sheet.frame_count {
            let measured = pointer_angle(image, sheet, k);
            let expected = sheet.frame_angle_deg(k);
            assert!(
                angle_diff(measured, expected).abs() <= POINTER_TOLERANCE_DEG,
                "{key} frame {k}: pointer at {measured:.1}°, manifest says {expected:.1}°"
            );
        }
    }
}

/// Angular profile of the panel in a ring around `control`: per 0.5° bin, the mean luminance
/// difference from the ring's median (ticks are dark on the cream panel, light on the dark one).
fn tick_profile(panel: &RgbaImage, scale: f32, control: &ControlLayout) -> Vec<(f32, f32)> {
    let r_in = control.diameter / 2.0 + 8.0;
    let r_out = control.diameter / 2.0 + 20.0;
    let mut bins: Vec<(f32, Vec<f32>)> = (0..720)
        .map(|i| (i as f32 * 0.5 - 180.0, Vec::new()))
        .collect();
    for (angle, samples) in &mut bins {
        let (s, c) = angle.to_radians().sin_cos();
        let mut r = r_in;
        while r <= r_out {
            let x = (control.center.x + r * s) * scale;
            let y = (control.center.y - r * c) * scale;
            samples.push(luminance(panel.pixel(x as u32, y as u32)));
            r += 0.25;
        }
    }
    let mut all: Vec<f32> = bins.iter().flat_map(|(_, s)| s.iter().copied()).collect();
    all.sort_by(f32::total_cmp);
    let median = all[all.len() / 2];
    bins.into_iter()
        .map(|(a, s)| {
            (
                a,
                s.iter().map(|l| (l - median).abs()).sum::<f32>() / s.len() as f32,
            )
        })
        .collect()
}

/// Center of the strongest tick within ±12° of `target`.
fn tick_near(profile: &[(f32, f32)], target: f32) -> f32 {
    let window: Vec<(f32, f32)> = profile
        .iter()
        .copied()
        .filter(|(a, _)| angle_diff(*a, target).abs() <= 12.0)
        .collect();
    let peak = window.iter().map(|(_, s)| *s).fold(0.0, f32::max);
    assert!(peak > 20.0, "no tick near {target}°");
    let (mut sum_w, mut sum_a) = (0.0, 0.0);
    for (a, s) in window.into_iter().filter(|(_, s)| *s >= 0.5 * peak) {
        sum_w += s;
        sum_a += s * angle_diff(a, target);
    }
    target + sum_a / sum_w
}

#[test]
fn panel_scale_ends_match_the_sprite_range() {
    let manifest = SpriteManifest::embedded();
    let panel = decode_png(PANEL_PNG).unwrap();
    for control in CONTROLS {
        let sheet = manifest.sheet(control.sprite);
        let profile = tick_profile(&panel, manifest.scale, &control);
        for (target, end) in [(sheet.angle_min_deg, "min"), (sheet.angle_max_deg, "max")] {
            let tick = tick_near(&profile, target);
            assert!(
                angle_diff(tick, target).abs() <= TICK_TOLERANCE_DEG,
                "{}: {end} tick at {tick:.1}°, sprite {end} at {target}°",
                control.param_id
            );
        }
    }
}

#[test]
fn drive_zero_db_points_at_the_first_tick() {
    // Drive's range starts at 0 dB, so 0 dB is v = 0 → frame 0, which must sit on the first tick.
    let manifest = SpriteManifest::embedded();
    let panel = decode_png(PANEL_PNG).unwrap();
    let sheet = manifest.sheet(DRIVE.sprite);
    let k = super::sprites::frame_index(0.0, sheet.frame_count);
    let tick = tick_near(
        &tick_profile(&panel, manifest.scale, &DRIVE),
        sheet.angle_min_deg,
    );
    assert!(angle_diff(sheet.frame_angle_deg(k), tick).abs() <= TICK_TOLERANCE_DEG);
}
