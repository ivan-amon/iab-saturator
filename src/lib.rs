use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

mod dsp;
mod editor;

use dsp::{curve::curve, db::db_to_gain, drive::drive, smoother::Smoother};

/// Time constant of the Drive smoother, τ in ms.
const DRIVE_SMOOTHING_TAU_MS: f32 = 10.0;

struct IabSaturator {
    params: Arc<IabSaturatorParams>,
    /// One shared smoother for the linear drive gain (Drive is a single parameter for all
    /// channels). Persists across buffers.
    drive_smoother: Smoother,
}

/// Parameter IDs are part of saved projects and automation: never change them.
#[derive(Params)]
pub struct IabSaturatorParams {
    /// Editor window size, persisted with the plugin state.
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    /// Drive in dB. Smoothing is done by our own `Smoother` on the linear gain, so nih-plug's
    /// built-in smoother is left disabled (the `FloatParam::new` default).
    #[id = "drive"]
    pub drive: FloatParam,

    /// Output level in dB. Declared, not wired to the DSP yet (no smoother until it is).
    #[id = "output"]
    pub output: FloatParam,

    /// Dry/wet mix in %. Declared, not wired to the DSP yet (no smoother until it is).
    #[id = "dry_wet"]
    pub dry_wet: FloatParam,

    /// Waveshaper curve. Declared, not wired to the DSP yet.
    #[id = "curve"]
    pub curve: EnumParam<CurveType>,
}

/// Curve selector positions. The `#[id]`s are stored in saved projects: never change them.
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    #[id = "tanh"]
    #[name = "TANH"]
    Tanh,
    #[id = "curve2"]
    #[name = "CURVE 2"]
    Curve2,
    #[id = "curve3"]
    #[name = "CURVE 3"]
    Curve3,
    #[id = "curve4"]
    #[name = "CURVE 4"]
    Curve4,
}

impl Default for IabSaturator {
    fn default() -> Self {
        Self {
            params: Arc::new(IabSaturatorParams::default()),
            drive_smoother: Smoother::default(),
        }
    }
}

impl Default for IabSaturatorParams {
    fn default() -> Self {
        let (window_width, window_height) = editor::WINDOW_SIZE;
        Self {
            editor_state: EguiState::from_size(window_width, window_height),

            drive: FloatParam::new(
                "Drive",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 36.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            output: FloatParam::new(
                "Output",
                0.0,
                FloatRange::Linear {
                    min: -36.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            dry_wet: FloatParam::new(
                "Dry/Wet",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            curve: EnumParam::new("Curve", CurveType::Tanh),
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

    // nih-plug splits the host buffer at automation points and calls `process()` once per
    // sub-block, so "read Drive once per process() call" stays sample accurate.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // fs is only needed to compute the smoother coefficient. Nothing else to allocate.
        self.drive_smoother
            .set_sample_rate(buffer_config.sample_rate, DRIVE_SMOOTHING_TAU_MS);
        true
    }

    fn reset(&mut self) {
        // Audio thread: no allocations. Snap gs to the current g so there is no ramp.
        self.drive_smoother
            .reset(db_to_gain(self.params.drive.value()));
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Read Drive once per buffer: g[n] is a staircase, the smoother makes it continuous.
        let g = db_to_gain(self.params.drive.value());

        // Per frame, then per channel. Time order within each channel is preserved.
        for channel_samples in buffer.iter_samples() {
            let gs = self.drive_smoother.next(g);
            for sample in channel_samples {
                *sample = curve(drive(gs, *sample));
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for IabSaturator {
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

nih_export_clap!(IabSaturator);

/// `nih_export_vst3!` expands nih-plug's `vst3_com::vtable` macro, which trips the
/// `semicolon_in_expressions_from_non_local_macros` future-compat lint at this call site.
/// Upstream code, not ours: allow that one lint only here so `clippy -D warnings` passes.
/// The `#[no_mangle]` entry points are still exported from inside this module.
#[allow(semicolon_in_expressions_from_non_local_macros)]
mod vst3_export {
    use super::IabSaturator;
    use nih_plug::prelude::*;

    nih_export_vst3!(IabSaturator);
}
