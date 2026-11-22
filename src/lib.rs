use nice_plug::editor::dpi::LogicalSize;
use nice_plug::prelude::*;
use nice_plug_egui::{
    EguiEditor, EguiEditorState, EguiNiceSettings, RepaintNotifier, create_egui_editor,
};
use std::sync::Arc;

mod dsp;
mod editor;
mod params;

use dsp::{curve::curve, db::db_to_gain, drive::drive, output::output, smoother::Smoother};
use editor::{IabEditor, Layout};
use params::IabSaturatorParams;

use crate::dsp::mix::mix;

/// Time constant of the smoother, τ in ms.
const SMOOTHING_TAU_MS: f32 = 10.0;

struct IabSaturator {
    params: Arc<IabSaturatorParams>,
    /// One shared smoother for the linear drive gain (Drive is a single parameter for all
    /// channels). Persists across buffers.
    drive_smoother: Smoother,
    output_smoother: Smoother,
    mix_smoother: Smoother,
    /// Window size and open/closed state, shared with nice-plug-egui.
    editor_state: Arc<EguiEditorState>,
    /// Handed to nice-plug-egui by `editor()`. `None` if layout.json failed to parse: the
    /// plugin then runs with the host's generic view.
    editor: Option<IabEditor>,
}

impl Default for IabSaturator {
    fn default() -> Self {
        let params = Arc::new(IabSaturatorParams::default());
        let editor = match Layout::load() {
            Ok(layout) => Some(IabEditor::new(params.clone(), layout)),
            Err(e) => {
                nice_error!("layout.json failed to parse: {e}");
                None
            }
        };
        let size = editor
            .as_ref()
            .map_or(LogicalSize::new(1.0, 1.0), IabEditor::window_size);
        Self {
            params,
            drive_smoother: Smoother::default(),
            output_smoother: Smoother::default(),
            mix_smoother: Smoother::default(),
            editor_state: EguiEditorState::from_size(size, 1.0),
            editor,
        }
    }
}

impl Plugin for IabSaturator {
    const NAME: &'static str = "IAB-Saturator";
    const VENDOR: &'static str = "Iván Amón";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "ivan.amon.dev@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first layout is the default.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    // nice-plug splits the host buffer at automation points and calls `process()` once per
    // sub-block, so "read Drive once per process() call" stays sample accurate.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type Editor = EguiEditor<IabEditor>;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Self::Editor> {
        create_egui_editor(
            self.editor_state.clone(),
            RepaintNotifier::new(),
            EguiNiceSettings::new().with_tile(Self::NAME),
            self.editor.take()?,
        )
    }

    fn activate(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        // fs is only needed to compute the smoother coefficient. Nothing else to allocate.
        self.drive_smoother
            .set_sample_rate(buffer_config.sample_rate, SMOOTHING_TAU_MS);
        self.output_smoother
            .set_sample_rate(buffer_config.sample_rate, SMOOTHING_TAU_MS);
        self.mix_smoother
            .set_sample_rate(buffer_config.sample_rate, SMOOTHING_TAU_MS);
        true
    }

    fn reset(&mut self) {
        // Audio thread: no allocations. Snap gs to the current g so there is no ramp.
        self.drive_smoother
            .reset(db_to_gain(self.params.drive.value()));
        self.output_smoother
            .reset(db_to_gain(self.params.output.value()));
        self.mix_smoother.reset(self.params.dry_wet.value() / 100.0);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Read Drive once per buffer: g[n] is a staircase, the smoother makes it continuous.
        let g = db_to_gain(self.params.drive.value());
        let g_out = db_to_gain(self.params.output.value());
        let m = self.params.dry_wet.value() / 100.0; // Because m in the GUI it's in %

        // Per frame, then per channel. Time order within each channel is preserved.
        for channel_samples in buffer.iter_samples() {
            let gs = self.drive_smoother.next(g);
            let g_out_s = self.output_smoother.next(g_out);
            let ms = self.mix_smoother.next(m);
            for sample in channel_samples {
                *sample = output(g_out_s, mix(ms, *sample, curve(drive(gs, *sample))));
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for IabSaturator {
    // Never change it: hosts use it to find the plugin in saved projects.
    const CLAP_ID: &'static str = "com.ivanamon.iab-saturator";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Mono-curve tanh saturator");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Distortion,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for IabSaturator {
    // Random, generated once. Never change it: hosts use it to find the plugin in saved projects.
    const VST3_CLASS_ID: [u8; 16] = [
        0x88, 0x9d, 0xa1, 0xd6, 0xb4, 0x1b, 0x33, 0x4d, 0x83, 0xf5, 0x70, 0x19, 0xa8, 0x10, 0x72,
        0x3d,
    ];

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

nice_export_clap!(IabSaturator);
nice_export_vst3!(IabSaturator);
