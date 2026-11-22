//! Pure GUI math: rectangles, angles and the mapping from mouse input to parameter values.
//! No egui state, no parameters: everything here is unit-tested.

use egui::{Pos2, Rect, Vec2, pos2, vec2};

/// Vertical drag distance that sweeps a knob over its full range ("200 px = full range" in the
/// `interaction` block of layout.json).
pub const DRAG_FULL_RANGE_PX: f32 = 200.0;

/// Shift scales drag and wheel movement by this factor ("Shift = fine (x0.1)" in layout.json).
pub const FINE_FACTOR: f32 = 0.1;

/// Normalized change per mouse-wheel notch on a continuous knob.
pub const WHEEL_STEP: f32 = 0.01;

/// Rectangle of size (w, h) centred on (cx, cy).
pub fn centered_rect(cx: f32, cy: f32, w: f32, h: f32) -> Rect {
    Rect::from_center_size(pos2(cx, cy), vec2(w, h))
}

/// Pointer angle in degrees (0 = 12 o'clock, clockwise positive) for a normalized value.
pub fn knob_angle_deg(from: f32, to: f32, normalized: f32) -> f32 {
    from + (to - from) * normalized
}

/// New normalized value after a vertical drag of `dy` px (screen y grows down, so dragging up
/// increases the value). Clamped to [0, 1].
pub fn drag_normalized(current: f32, dy: f32, fine: bool) -> f32 {
    let scale = if fine { FINE_FACTOR } else { 1.0 };
    (current - dy / DRAG_FULL_RANGE_PX * scale).clamp(0.0, 1.0)
}

/// New normalized value after `notches` mouse-wheel notches (positive = up). Clamped to [0, 1].
pub fn wheel_normalized(current: f32, notches: f32, fine: bool) -> f32 {
    let scale = if fine { FINE_FACTOR } else { 1.0 };
    (current + notches * WHEEL_STEP * scale).clamp(0.0, 1.0)
}

/// Index of the discrete position closest to `normalized`, for `count` positions.
pub fn index_from_normalized(normalized: f32, count: usize) -> usize {
    let last = count.saturating_sub(1);
    ((normalized.clamp(0.0, 1.0) * last as f32).round() as usize).min(last)
}

/// Normalized value of position `index`, for `count` positions.
pub fn normalized_from_index(index: usize, count: usize) -> f32 {
    let last = count.saturating_sub(1);
    if last == 0 {
        0.0
    } else {
        index.min(last) as f32 / last as f32
    }
}

/// `index` moved by `steps`, clamped to the valid positions.
pub fn step_index(index: usize, steps: i32, count: usize) -> usize {
    let last = count.saturating_sub(1) as i64;
    (index as i64 + steps as i64).clamp(0, last) as usize
}

/// The next position, wrapping from the last one back to the first.
pub fn next_index_wrapping(index: usize, count: usize) -> usize {
    if count == 0 { 0 } else { (index + 1) % count }
}

/// Splits an accumulated drag distance into whole steps of `px_per_step` (dragging up = positive
/// steps) and the remainder that is carried to the next frame.
pub fn drag_steps(accumulated_dy: f32, px_per_step: f32) -> (i32, f32) {
    // Dragging up (negative dy) steps up.
    let steps = (-accumulated_dy / px_per_step).trunc();
    (steps as i32, accumulated_dy + steps * px_per_step)
}

/// Top-left corner for a single-line text of `width`, horizontally centred on `cx`, with the
/// cap-height block vertically centred on `cy` (the layout.json text convention). `baseline` is
/// the baseline's distance from the top of the laid-out text.
pub fn text_top_left(cx: f32, cy: f32, width: f32, baseline: f32, cap_height: f32) -> Pos2 {
    // The cap block spans [baseline - cap_height, baseline] below the top.
    pos2(cx - width / 2.0, cy + cap_height / 2.0 - baseline)
}

/// Size at 1x of a sprite whose PNG is `png_size` px (sprites are rendered at `render_scale`).
pub fn sprite_size(png_size: [usize; 2], render_scale: f32) -> Vec2 {
    vec2(png_size[0] as f32, png_size[1] as f32) / render_scale
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    #[test]
    fn centered_rect_is_centred() {
        let r = centered_rect(520.0, 238.0, 216.0, 216.0);
        assert_eq!(r.center(), pos2(520.0, 238.0));
        assert_eq!(r.size(), vec2(216.0, 216.0));
        assert_eq!(r.min, pos2(412.0, 130.0));
    }

    #[test]
    fn knob_angle_spans_the_sweep() {
        assert!((knob_angle_deg(-135.0, 135.0, 0.0) + 135.0).abs() < EPS);
        assert!(knob_angle_deg(-135.0, 135.0, 0.5).abs() < EPS);
        assert!((knob_angle_deg(-135.0, 135.0, 1.0) - 135.0).abs() < EPS);
    }

    #[test]
    fn dragging_up_the_full_range_goes_from_min_to_max() {
        assert!((drag_normalized(0.0, -DRAG_FULL_RANGE_PX, false) - 1.0).abs() < EPS);
        assert!((drag_normalized(1.0, DRAG_FULL_RANGE_PX, false)).abs() < EPS);
        assert!((drag_normalized(0.5, -20.0, false) - 0.6).abs() < EPS);
    }

    #[test]
    fn shift_drag_is_ten_times_finer() {
        assert!((drag_normalized(0.5, -20.0, true) - 0.51).abs() < EPS);
    }

    #[test]
    fn drag_is_clamped() {
        assert_eq!(drag_normalized(0.9, -1000.0, false), 1.0);
        assert_eq!(drag_normalized(0.1, 1000.0, false), 0.0);
    }

    #[test]
    fn wheel_steps_and_clamps() {
        assert!((wheel_normalized(0.5, 1.0, false) - (0.5 + WHEEL_STEP)).abs() < EPS);
        assert!((wheel_normalized(0.5, -2.0, false) - (0.5 - 2.0 * WHEEL_STEP)).abs() < EPS);
        assert!((wheel_normalized(0.5, 1.0, true) - (0.5 + WHEEL_STEP * FINE_FACTOR)).abs() < EPS);
        assert_eq!(wheel_normalized(1.0, 3.0, false), 1.0);
        assert_eq!(wheel_normalized(0.0, -3.0, false), 0.0);
    }

    #[test]
    fn index_and_normalized_round_trip() {
        for i in 0..7 {
            assert_eq!(index_from_normalized(normalized_from_index(i, 7), 7), i);
        }
        assert_eq!(normalized_from_index(0, 4), 0.0);
        assert_eq!(normalized_from_index(3, 4), 1.0);
        assert_eq!(index_from_normalized(0.49, 3), 1);
        assert_eq!(index_from_normalized(1.7, 3), 2);
    }

    #[test]
    fn step_index_clamps() {
        assert_eq!(step_index(3, 1, 7), 4);
        assert_eq!(step_index(6, 1, 7), 6);
        assert_eq!(step_index(0, -1, 7), 0);
        assert_eq!(step_index(2, -5, 7), 0);
    }

    #[test]
    fn next_index_wraps() {
        assert_eq!(next_index_wrapping(0, 7), 1);
        assert_eq!(next_index_wrapping(6, 7), 0);
    }

    #[test]
    fn drag_steps_carries_the_remainder() {
        assert_eq!(drag_steps(-25.0, 10.0), (2, -5.0));
        assert_eq!(drag_steps(15.0, 10.0), (-1, 5.0));
        assert_eq!(drag_steps(-9.0, 10.0), (0, -9.0));
    }

    #[test]
    fn text_cap_block_is_centred() {
        // 40 px wide, baseline 12 px below the top, cap height 8 px: the cap block spans
        // [baseline - 8, baseline], so its centre is 4 px above the baseline.
        let p = text_top_left(100.0, 50.0, 40.0, 12.0, 8.0);
        assert_eq!(p, pos2(80.0, 50.0 + 4.0 - 12.0));
    }

    #[test]
    fn sprite_size_is_png_size_over_render_scale() {
        assert_eq!(sprite_size([432, 432], 2.0), vec2(216.0, 216.0));
        assert_eq!(sprite_size([88, 54], 2.0), vec2(44.0, 27.0));
    }
}
