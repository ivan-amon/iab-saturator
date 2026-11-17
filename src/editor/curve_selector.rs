//! Discrete rotary selector (CURVE): position maths (pure) and the egui widget.

use super::knob::{control_response, drag_began_inside, is_inside};
use super::layout::ControlLayout;
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::Ui;

/// Vertical drag distance, in logical px, per selector position (the knobs' 200 px full range
/// split over the 4 positions).
pub const DRAG_PX_PER_POSITION: f32 = 50.0;

/// Position after a click: the next one, wrapping from the last to the first.
pub fn next_position(position: usize, count: usize) -> usize {
    (position + 1) % count.max(1)
}

/// Position after a vertical drag of `dy` logical px from `start` (dragging up moves to higher
/// positions). Clamped, no wrapping.
pub fn position_after_drag(start: usize, dy: f32, count: usize) -> usize {
    let steps = (-dy / DRAG_PX_PER_POSITION).round();
    let last = count.saturating_sub(1) as f32;
    (start as f32 + steps).clamp(0.0, last) as usize
}

/// Normalized parameter value of a position: `i / (count − 1)`.
pub fn position_to_normalized(position: usize, count: usize) -> f32 {
    if count <= 1 {
        return 0.0;
    }
    position as f32 / (count - 1) as f32
}

/// Position of a normalized value: `round(v · (count − 1))`, clamped.
pub fn normalized_to_position(v: f32, count: usize) -> usize {
    let last = count.saturating_sub(1);
    let v = if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
    ((v * last as f32).round() as usize).min(last)
}

/// Selector input: click = next position (wrapping), vertical drag = step through positions,
/// double-click = default. A double-click's first click also cycles; the second resets.
/// Every change goes through `ParamSetter` so the host records automation and undo.
pub fn selector<P: Param>(ui: &mut Ui, setter: &ParamSetter, param: &P, layout: &ControlLayout) {
    let count = param.step_count().map_or(2, |steps| steps + 1);
    let response = control_response(ui, layout);
    // (position at drag start, accumulated vertical drag in px) while a drag is in progress.
    let drag_id = response.id.with("drag");
    let current = normalized_to_position(param.unmodulated_normalized_value(), count);
    let set = |position: usize| {
        setter.set_parameter_normalized(param, position_to_normalized(position, count));
    };

    if response.drag_started() && drag_began_inside(ui, layout) {
        setter.begin_set_parameter(param);
        ui.data_mut(|d| d.insert_temp(drag_id, (current, 0.0f32)));
    }
    if let Some((start, dy)) = ui.data(|d| d.get_temp::<(usize, f32)>(drag_id)) {
        let dy = dy + response.drag_delta().y;
        ui.data_mut(|d| d.insert_temp(drag_id, (start, dy)));
        let position = position_after_drag(start, dy, count);
        if position != current {
            set(position);
        }
        if response.drag_stopped() {
            ui.data_mut(|d| d.remove::<(usize, f32)>(drag_id));
            setter.end_set_parameter(param);
        }
        return;
    }

    if !is_inside(layout, response.interact_pointer_pos()) {
        return;
    }
    // egui reports the second click of a double-click as both clicked and double-clicked.
    let target = if response.double_clicked() {
        normalized_to_position(param.default_normalized_value(), count)
    } else if response.clicked() {
        next_position(current, count)
    } else {
        return;
    };
    setter.begin_set_parameter(param);
    set(target);
    setter.end_set_parameter(param);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_cycles_and_wraps() {
        assert_eq!(next_position(0, 4), 1);
        assert_eq!(next_position(2, 4), 3);
        assert_eq!(next_position(3, 4), 0);
    }

    #[test]
    fn drag_steps_every_50_px_and_clamps() {
        assert_eq!(position_after_drag(0, -24.0, 4), 0);
        assert_eq!(position_after_drag(0, -26.0, 4), 1);
        assert_eq!(position_after_drag(0, -100.0, 4), 2);
        assert_eq!(position_after_drag(1, 50.0, 4), 0);
        assert_eq!(position_after_drag(0, -1000.0, 4), 3);
        assert_eq!(position_after_drag(3, 1000.0, 4), 0);
    }

    #[test]
    fn normalized_round_trip() {
        for i in 0..4 {
            assert_eq!(normalized_to_position(position_to_normalized(i, 4), 4), i);
        }
        assert_eq!(position_to_normalized(1, 4), 1.0 / 3.0);
        assert_eq!(normalized_to_position(0.49, 4), 1);
        assert_eq!(normalized_to_position(1.2, 4), 3);
        assert_eq!(normalized_to_position(-0.2, 4), 0);
    }
}
