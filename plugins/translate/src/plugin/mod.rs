use crate::dsp::TranslateProcessor;
use crate::ir::IrState;
use crate::params::TranslateParams;
use crate::ui;
use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct TranslatePlugin {
    params: Arc<TranslateParams>,
    processor: TranslateProcessor,
    ir_state: IrState,
}

impl Default for TranslatePlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(TranslateParams::default()),
            processor: TranslateProcessor::default(),
            ir_state: IrState::default(),
        }
    }
}

impl Plugin for TranslatePlugin {
    const NAME: &'static str = "TRANSLATE";
    const VENDOR: &'static str = "Placeholder Vendor";
    const URL: &'static str = "https://example.com/translate";
    const EMAIL: &'static str = "support@example.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

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

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        let params: Arc<dyn Params> = self.params.clone();
        params
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create_editor(self.params.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let _ = &self.ir_state;
        self.processor.prepare(buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.processor.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.processor.process(buffer);
        ProcessStatus::Normal
    }
}

impl ClapPlugin for TranslatePlugin {
    const CLAP_ID: &'static str = "com.placeholdervendor.translate";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Mix translation checking plugin scaffold");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for TranslatePlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"TranslatePlugin!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
