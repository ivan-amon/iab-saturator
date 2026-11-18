//! Mouse input → parameter changes, always through `ParamSetter` (begin / set / end per
//! gesture) so the host records automation and undo. Nothing is drawn here.

use super::geometry::{
    DRAG_FULL_RANGE_PX, centered_rect, drag_normalized, drag_steps, index_from_normalized,
    next_index_wrapping, normalized_from_index, step_index, wheel_normalized,
};
use egui::{Event, Id, Pos2, Rect, Response, Sense, Ui};
use nice_plug::prelude::{Param, ParamSetter};

/// Continuous knob: vertical drag (Shift = fine), double-click = default, wheel steps.
pub fn knob<P: Param>(ui: &Ui, setter: &ParamSetter, param: &P, id: Id, centre: Pos2, d: f32) {
    let response = ui.interact(
        centered_rect(centre.x, centre.y, d, d),
        id,
        Sense::click_and_drag(),
    );
    let fine = ui.input(|i| i.modifiers.shift);

    if response.double_clicked() {
        set_once(setter, param, param.default_normalized_value());
        return;
    }
    if response.drag_started() {
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        let dy = response.drag_delta().y;
        if dy != 0.0 {
            let value = drag_normalized(param.unmodulated_normalized_value(), dy, fine);
            setter.set_parameter_normalized(param, value);
        }
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }

    let notches = wheel_notches(ui, &response);
    if notches != 0 {
        let value = wheel_normalized(param.unmodulated_normalized_value(), notches as f32, fine);
        set_once(setter, param, value);
    }
}

/// Discrete selector with `count` positions: click = next (wraps), vertical drag and wheel
/// step (clamped), clicking a position's hit area selects it. `drag_remainder` carries the
/// drag distance that has not produced a step yet.
#[allow(clippy::too_many_arguments)]
pub fn selector<P: Param>(
    ui: &Ui,
    setter: &ParamSetter,
    param: &P,
    id: Id,
    centre: Pos2,
    d: f32,
    count: usize,
    position_hits: impl Iterator<Item = (usize, Rect)>,
    drag_remainder: &mut f32,
) {
    let current = index_from_normalized(param.unmodulated_normalized_value(), count);
    let response = ui.interact(
        centered_rect(centre.x, centre.y, d, d),
        id,
        Sense::click_and_drag(),
    );

    if response.clicked() {
        set_index(setter, param, next_index_wrapping(current, count), count);
    }
    if response.drag_started() {
        *drag_remainder = 0.0;
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        let px_per_step = DRAG_FULL_RANGE_PX / (count.saturating_sub(1).max(1)) as f32;
        let (steps, remainder) = drag_steps(*drag_remainder + response.drag_delta().y, px_per_step);
        *drag_remainder = remainder;
        if steps != 0 {
            let index = step_index(current, steps, count);
            setter.set_parameter_normalized(param, normalized_from_index(index, count));
        }
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }

    let notches = wheel_notches(ui, &response);
    if notches != 0 {
        set_index(setter, param, step_index(current, notches, count), count);
    }

    for (index, rect) in position_hits {
        if ui.interact(rect, id.with(index), Sense::click()).clicked() {
            set_index(setter, param, index, count);
        }
    }
}

/// Segmented buttons: clicking segment `index` selects it.
pub fn segments<P: Param>(
    ui: &Ui,
    setter: &ParamSetter,
    param: &P,
    id: Id,
    count: usize,
    hits: impl Iterator<Item = (usize, Rect)>,
) {
    for (index, rect) in hits {
        if ui.interact(rect, id.with(index), Sense::click()).clicked() {
            set_index(setter, param, index, count);
        }
    }
}

/// On/off toggle: a click flips it.
pub fn toggle<P: Param>(ui: &Ui, setter: &ParamSetter, param: &P, id: Id, hit: Rect) {
    if ui.interact(hit, id, Sense::click()).clicked() {
        let on = param.unmodulated_normalized_value() >= 0.5;
        set_once(setter, param, if on { 0.0 } else { 1.0 });
    }
}

/// A complete one-shot gesture (click, wheel notch).
fn set_once<P: Param>(setter: &ParamSetter, param: &P, normalized: f32) {
    setter.begin_set_parameter(param);
    setter.set_parameter_normalized(param, normalized);
    setter.end_set_parameter(param);
}

fn set_index<P: Param>(setter: &ParamSetter, param: &P, index: usize, count: usize) {
    set_once(setter, param, normalized_from_index(index, count));
}

/// Wheel notches over `response` this frame (positive = up). One per wheel event, whatever its
/// unit, so a mouse notch is always one step.
fn wheel_notches(ui: &Ui, response: &Response) -> i32 {
    if !response.hovered() || response.dragged() {
        return 0;
    }
    ui.input(|i| {
        i.events
            .iter()
            .map(|e| match e {
                Event::MouseWheel { delta, .. } if delta.y > 0.0 => 1,
                Event::MouseWheel { delta, .. } if delta.y < 0.0 => -1,
                _ => 0,
            })
            .sum()
    })
}
