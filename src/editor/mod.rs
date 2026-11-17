//! Plugin editor: an image-based panel drawn with egui. See CLAUDE.md (GUI rules, layout).
//!
//! The editor never touches DSP state: it reads parameters and writes them through
//! `ParamSetter` only.

use crate::IabSaturatorParams;
use assets::DecodedAssets;
use nih_plug::prelude::{Editor, ParamSetter};
use nih_plug_egui::egui::{CentralPanel, Context, Frame};
use paint::{paint, ControlView, Gfx};
use std::sync::Arc;

pub mod assets;
pub mod curve_selector;
pub mod fonts;
pub mod knob;
pub mod layout;
pub mod paint;
pub mod sprites;
mod window;

#[cfg(test)]
mod alignment_test;
#[cfg(test)]
mod preview_test;

/// Editor window size in logical px.
pub const WINDOW_SIZE: (u32, u32) = (600, 360);

/// Frames rendered unconditionally after the window opens. Some hosts (Ableton Live among them)
/// open the editor while its window is still hidden, and a frame drawn then is lost. 30 frames
/// is about half a second at baseview's 60 Hz frame timer.
const FORCED_REPAINT_FRAMES: u32 = 30;

pub fn create(params: Arc<IabSaturatorParams>) -> Option<Box<dyn Editor>> {
    Some(Box::new(window::IabEditor::new(params)))
}

/// Current views of the controls, in `layout::CONTROLS` order.
fn current_views(p: &IabSaturatorParams) -> [ControlView; 4] {
    [
        ControlView::current(&p.drive),
        ControlView::current(&p.dry_wet),
        ControlView::current(&p.output),
        ControlView::current(&p.curve),
    ]
}

/// State of one open editor window.
struct EditorUi {
    params: Arc<IabSaturatorParams>,
    /// Decoded images, until they are uploaded as textures on the first frame.
    assets: Option<DecodedAssets>,
    gfx: Option<Gfx>,
    /// What the last frame showed; a repaint is requested only when this changes.
    last_views: Option<[ControlView; 4]>,
    forced_repaints_left: u32,
}

impl EditorUi {
    fn new(params: Arc<IabSaturatorParams>) -> Self {
        Self {
            params,
            assets: Some(DecodedAssets::decode()),
            gfx: None,
            last_views: None,
            forced_repaints_left: FORCED_REPAINT_FRAMES,
        }
    }

    /// Once, when the window opens.
    fn build(&mut self, ctx: &Context) {
        ctx.set_fonts(fonts::font_definitions());
    }

    /// Every frame (egui-baseview runs the pass every frame; it renders only on request).
    fn update(&mut self, ctx: &Context, setter: &ParamSetter) {
        // Textures are uploaded inside a pass: only then does egui know the GPU's texture limit.
        if self.gfx.is_none() {
            let Some(assets) = self.assets.take() else {
                return;
            };
            if Gfx::largest_side(&assets) as usize > ctx.input(|i| i.max_texture_side) {
                // The GPU cannot hold the sprite sheets. Leave the window blank rather than crash
                // the host. Not expected on any GL 3.2 desktop GPU (4096 px is needed).
                return;
            }
            self.gfx = Some(Gfx::upload(ctx, &assets));
        }
        let Some(gfx) = &self.gfx else {
            return;
        };

        let p = &self.params;
        CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
            knob::knob(ui, setter, &p.drive, &layout::DRIVE);
            knob::knob(ui, setter, &p.dry_wet, &layout::DRY_WET);
            knob::knob(ui, setter, &p.output, &layout::OUTPUT);
            curve_selector::selector(ui, setter, &p.curve, &layout::CURVE);

            // Read after the input handling, so this frame already shows the new values.
            let views = current_views(p);
            paint(ui.painter(), gfx, &views);

            // Parameters also change from the host (automation, undo, presets): repaint when
            // anything shown differs from the last frame. Input events repaint on their own.
            if self.forced_repaints_left > 0 || self.last_views.as_ref() != Some(&views) {
                ctx.request_repaint();
            }
            self.forced_repaints_left = self.forced_repaints_left.saturating_sub(1);
            self.last_views = Some(views);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug::prelude::Params;

    #[test]
    fn views_follow_the_layout_order() {
        let ids: Vec<&str> = layout::CONTROLS.iter().map(|c| c.param_id).collect();
        assert_eq!(ids, ["drive", "dry_wet", "output", "curve"]);
        // `current_views` reads drive, dry_wet, output, curve in that order: check the readouts
        // at the defaults land on the right controls.
        let views = current_views(&IabSaturatorParams::default());
        let readouts: Vec<&str> = views.iter().map(|v| v.readout.as_str()).collect();
        assert_eq!(readouts, ["0.0 dB", "100 %", "0.0 dB", "TANH"]);
    }

    #[test]
    fn layout_ids_are_real_parameter_ids() {
        let params = IabSaturatorParams::default();
        let ids: Vec<String> = params
            .param_map()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        for c in layout::CONTROLS {
            assert!(ids.iter().any(|id| id == c.param_id), "{}", c.param_id);
        }
    }

    #[test]
    fn parameter_ids_are_stable() {
        // IDs are stored in saved projects and automation: this list must never change.
        let params = IabSaturatorParams::default();
        let ids: Vec<String> = params
            .param_map()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        assert_eq!(ids, ["drive", "output", "dry_wet", "curve"]);
    }

    #[test]
    fn readouts_use_each_parameters_formatter() {
        let p = IabSaturatorParams::default();
        let at = |v: ControlView| v.readout;
        assert_eq!(at(ControlView::of(&p.drive, 15.0 / 36.0)), "15.0 dB");
        assert_eq!(at(ControlView::of(&p.output, 30.0 / 36.0)), "-6.0 dB");
        assert_eq!(at(ControlView::of(&p.dry_wet, 0.75)), "75 %");
        assert_eq!(at(ControlView::of(&p.curve, 2.0 / 3.0)), "CURVE 3");
    }
}
