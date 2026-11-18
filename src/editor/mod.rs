//! The plugin editor (nice-plug-egui). Reads and writes parameters only; never touches DSP
//! state. Positions, sizes and colours come from layout.json.

mod assets;
mod fonts;
mod geometry;
mod layout;
mod paint;
mod widgets;

use crate::params::IabSaturatorParams;
use assets::{CURVE_COUNT, OVERSAMPLING_COUNT};
use egui::{Rect, pos2, vec2};
use geometry::centered_rect;
use nice_plug::context::gui::GuiContext;
use nice_plug::editor::dpi::LogicalSize;
use nice_plug::prelude::*;
use nice_plug_egui::{Frame, NiceEguiApp, baseview::HandlerError};
use std::sync::Arc;

pub use layout::Layout;

/// Editor state that lives as long as the plugin (the window can open and close many times).
pub struct IabEditor {
    params: Arc<IabSaturatorParams>,
    layout: Layout,
    open: Option<OpenEditor>,
    /// Selector drag distance not yet turned into a step.
    selector_drag_remainder: f32,
}

/// State that only exists while a window is open.
struct OpenEditor {
    gui: GuiContext,
    /// `None` if the assets failed to load (logged once); the window then stays empty.
    textures: Option<paint::Textures>,
}

impl IabEditor {
    pub fn new(params: Arc<IabSaturatorParams>, layout: Layout) -> Self {
        Self {
            params,
            layout,
            open: None,
            selector_drag_remainder: 0.0,
        }
    }

    /// Fixed window size (layout.json `meta.panel`).
    pub fn window_size(&self) -> LogicalSize<f32> {
        LogicalSize::new(self.layout.meta.panel.w, self.layout.meta.panel.h)
    }

    fn handle_input(&mut self, ui: &egui::Ui, setter: &ParamSetter) {
        let params = &*self.params;
        let l = &self.layout;

        for k in &l.knobs {
            if let Some(param) = paint::float_param(params, &k.param) {
                let id = ui.scope_id().with("knob").with(&k.id);
                widgets::knob(ui, setter, param, id, pos2(k.cx, k.cy), k.body_d);
            }
        }

        let sel = &l.selector;
        let led = sel.position_leds.sprite;
        let hits = sel.positions.iter().flat_map(|p| {
            let label = centered_rect(
                p.label_sprite.cx,
                p.label_sprite.cy,
                p.label_sprite.w,
                p.label_sprite.h,
            );
            let led = centered_rect(p.led.cx, p.led.cy, led.w, led.h);
            [(p.index, label), (p.index, led)]
        });
        widgets::selector(
            ui,
            setter,
            &params.curve,
            ui.scope_id().with("curve"),
            pos2(sel.cx, sel.cy),
            sel.body_d,
            CURVE_COUNT,
            hits,
            &mut self.selector_drag_remainder,
        );

        let os = &l.bottom_bar.oversampling;
        let hits = os.segments.iter().map(|s| {
            (
                s.index,
                paint::segment_rect(os.bounds.y, os.bounds.h, s.x, s.w),
            )
        });
        widgets::segments(
            ui,
            setter,
            &params.oversampling,
            ui.scope_id().with("oversampling"),
            OVERSAMPLING_COUNT,
            hits,
        );

        for t in &l.bottom_bar.toggles {
            if let Some(param) = paint::bool_param(params, &t.param) {
                let hit = Rect::from_min_size(pos2(t.hit.x, t.hit.y), vec2(t.hit.w, t.hit.h));
                widgets::toggle(
                    ui,
                    setter,
                    param,
                    ui.scope_id().with("toggle").with(&t.id),
                    hit,
                );
            }
        }
    }
}

impl NiceEguiApp for IabEditor {
    fn build(
        &mut self,
        egui_ctx: egui::Context,
        gui: GuiContext,
        _frame: &mut Frame,
    ) -> Result<(), HandlerError> {
        fonts::install(&egui_ctx);
        // Fixed-size panel: no Ctrl +/- zoom.
        egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let textures = match paint::Textures::load(&egui_ctx, &self.layout) {
            Ok(textures) => Some(textures),
            Err(e) => {
                nice_error!("GUI assets failed to load: {e}");
                None
            }
        };
        self.open = Some(OpenEditor { gui, textures });
        Ok(())
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        let Some(open) = self.open.take() else {
            return;
        };
        if let Some(textures) = &open.textures {
            let setter = open.gui.param_setter();
            // Input first, so this frame already paints the new values.
            self.handle_input(ui, &setter);
            paint::paint(ui.painter(), &self.layout, textures, &self.params);
        }
        self.open = Some(open);
    }

    fn editor_closed(&mut self) {
        self.open = None;
    }
}
