//! Rotary knob: interaction maths (pure) and the egui widget.
//!
//! The widget only handles input. Drawing is done by `paint`, from the parameter value.

use super::layout::ControlLayout;
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{Event, Id, MouseWheelUnit, Pos2, Rect, Response, Sense, Ui, Vec2};

/// Vertical drag distance, in logical px, that sweeps the full normalized range.
pub const DRAG_FULL_RANGE_PX: f32 = 200.0;
/// Shift slows dragging down by this factor.
pub const FINE_DRAG_DIVISOR: f32 = 10.0;
/// Normalized change per mouse-wheel notch.
pub const WHEEL_STEP: f32 = 0.01;
/// Normalized change per mouse-wheel notch with Shift held.
pub const WHEEL_STEP_FINE: f32 = 0.001;

/// New normalized value after a vertical drag of `dy` logical px (screen y grows downwards, so
/// dragging up increases the value). Clamped to [0, 1].
pub fn drag_to_normalized(v: f32, dy: f32, fine: bool) -> f32 {
    let divisor = if fine { FINE_DRAG_DIVISOR } else { 1.0 };
    (v - dy / (DRAG_FULL_RANGE_PX * divisor)).clamp(0.0, 1.0)
}

/// New normalized value after `notches` wheel notches (positive = up). Clamped to [0, 1].
pub fn wheel_to_normalized(v: f32, notches: f32, fine: bool) -> f32 {
    let step = if fine { WHEEL_STEP_FINE } else { WHEEL_STEP };
    (v + notches * step).clamp(0.0, 1.0)
}

/// Wheel notches in one egui wheel event (positive = up). Uses the vertical axis, or the
/// horizontal one when there is no vertical movement (some systems turn Shift+wheel into
/// horizontal scrolling). Pixel-precise deltas (touchpads) are converted with egui's line height.
pub fn wheel_notches(unit: MouseWheelUnit, delta: Vec2, points_per_line: f32) -> f32 {
    let d = if delta.y != 0.0 { delta.y } else { delta.x };
    match unit {
        MouseWheelUnit::Line | MouseWheelUnit::Page => d,
        MouseWheelUnit::Point => d / points_per_line,
    }
}

// ---------------------------------------------------------------------------------------------
// Widget

/// The interactive circle of a control.
pub(super) fn control_response(ui: &mut Ui, layout: &ControlLayout) -> Response {
    let rect = Rect::from_center_size(layout.center, Vec2::splat(layout.diameter));
    ui.interact(rect, Id::new(layout.param_id), Sense::click_and_drag())
}

/// Whether `pos` is inside the control's circle (the response rect is its bounding square).
pub(super) fn is_inside(layout: &ControlLayout, pos: Option<Pos2>) -> bool {
    pos.is_some_and(|p| p.distance(layout.center) <= layout.diameter / 2.0)
}

/// Whether the current drag began inside the control's circle.
pub(super) fn drag_began_inside(ui: &Ui, layout: &ControlLayout) -> bool {
    is_inside(layout, ui.input(|i| i.pointer.press_origin()))
}

/// Knob input: vertical drag (Shift = fine), double-click = default, wheel = steps.
/// Every change goes through `ParamSetter` so the host records automation and undo.
pub fn knob<P: Param>(ui: &mut Ui, setter: &ParamSetter, param: &P, layout: &ControlLayout) {
    let response = control_response(ui, layout);
    // Normalized value while a drag is in progress. Kept here rather than re-read from the
    // parameter so host-side rounding cannot make the drag drift.
    let drag_id = response.id.with("drag");
    let fine = ui.input(|i| i.modifiers.shift);

    if response.drag_started() && drag_began_inside(ui, layout) {
        setter.begin_set_parameter(param);
        let v = param.unmodulated_normalized_value();
        ui.data_mut(|d| d.insert_temp(drag_id, v));
    }
    if let Some(v) = ui.data(|d| d.get_temp::<f32>(drag_id)) {
        let dy = response.drag_delta().y;
        if dy != 0.0 {
            let v = drag_to_normalized(v, dy, fine);
            ui.data_mut(|d| d.insert_temp(drag_id, v));
            setter.set_parameter_normalized(param, v);
        }
        if response.drag_stopped() {
            ui.data_mut(|d| d.remove::<f32>(drag_id));
            setter.end_set_parameter(param);
        }
        return;
    }

    if response.double_clicked() && is_inside(layout, response.interact_pointer_pos()) {
        setter.begin_set_parameter(param);
        setter.set_parameter_normalized(param, param.default_normalized_value());
        setter.end_set_parameter(param);
        return;
    }

    if is_inside(layout, response.hover_pos()) {
        let points_per_line = ui.ctx().options(|o| o.line_scroll_speed);
        let notches: f32 = ui.input(|i| {
            i.events
                .iter()
                .map(|e| match e {
                    Event::MouseWheel { unit, delta, .. } => {
                        wheel_notches(*unit, *delta, points_per_line)
                    }
                    _ => 0.0,
                })
                .sum()
        });
        if notches != 0.0 {
            let v = wheel_to_normalized(param.unmodulated_normalized_value(), notches, fine);
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, v);
            setter.end_set_parameter(param);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn dragging_up_200_px_sweeps_the_full_range() {
        assert!(close(drag_to_normalized(0.0, -200.0, false), 1.0));
        assert!(close(drag_to_normalized(1.0, 200.0, false), 0.0));
        assert!(close(drag_to_normalized(0.5, -20.0, false), 0.6));
    }

    #[test]
    fn shift_drag_is_ten_times_slower() {
        assert!(close(drag_to_normalized(0.5, -20.0, true), 0.51));
        assert!(close(drag_to_normalized(0.0, -2000.0, true), 1.0));
    }

    #[test]
    fn drag_clamps_to_unit_range() {
        assert_eq!(drag_to_normalized(0.9, -100.0, false), 1.0);
        assert_eq!(drag_to_normalized(0.1, 100.0, false), 0.0);
    }

    #[test]
    fn wheel_steps_and_clamps() {
        assert!(close(wheel_to_normalized(0.5, 1.0, false), 0.51));
        assert!(close(wheel_to_normalized(0.5, -3.0, false), 0.47));
        assert!(close(wheel_to_normalized(0.5, 1.0, true), 0.501));
        assert_eq!(wheel_to_normalized(0.995, 1.0, false), 1.0);
        assert_eq!(wheel_to_normalized(0.0, -1.0, false), 0.0);
    }

    #[test]
    fn wheel_notches_by_unit() {
        use nih_plug_egui::egui::vec2;
        assert_eq!(
            wheel_notches(MouseWheelUnit::Line, vec2(0.0, 1.0), 40.0),
            1.0
        );
        assert_eq!(
            wheel_notches(MouseWheelUnit::Line, vec2(0.0, -2.0), 40.0),
            -2.0
        );
        assert_eq!(
            wheel_notches(MouseWheelUnit::Point, vec2(0.0, 20.0), 40.0),
            0.5
        );
        assert_eq!(
            wheel_notches(MouseWheelUnit::Page, vec2(0.0, -1.0), 40.0),
            -1.0
        );
        // Horizontal only (Shift+wheel on some systems).
        assert_eq!(
            wheel_notches(MouseWheelUnit::Line, vec2(1.0, 0.0), 40.0),
            1.0
        );
        // Vertical wins when both are present.
        assert_eq!(
            wheel_notches(MouseWheelUnit::Line, vec2(3.0, -1.0), 40.0),
            -1.0
        );
    }
}
