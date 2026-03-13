use crate::dsp::TranslateProcessor;
use crate::ir::IrState;
use crate::params::TranslateParams;
use crate::quick_cycle::QuickCycleShared;
use crate::ui;
use crate::workflow::WorkflowShared;
use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct TranslatePlugin {
    params: Arc<TranslateParams>,
    processor: TranslateProcessor,
    ir_state: IrState,
    quick_cycle: Arc<QuickCycleShared>,
    workflow: Arc<WorkflowShared>,
}

impl Default for TranslatePlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(TranslateParams::default()),
            processor: TranslateProcessor::default(),
            ir_state: IrState::default(),
            quick_cycle: Arc::new(QuickCycleShared::default()),
            workflow: Arc::new(WorkflowShared::default()),
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
        ui::create_editor(
            self.params.clone(),
            self.quick_cycle.clone(),
            self.workflow.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.ir_state
            .prepare_for_sample_rate(buffer_config.sample_rate);
        self.processor.prepare(
            buffer_config.sample_rate,
            &self.ir_state,
            self.params.as_ref(),
            self.quick_cycle.as_ref(),
            self.workflow.as_ref(),
        );
        true
    }

    fn reset(&mut self) {
        self.processor
            .reset(self.quick_cycle.as_ref(), self.workflow.as_ref());
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.processor.process(
            buffer,
            self.params.as_ref(),
            &self.ir_state,
            self.quick_cycle.as_ref(),
            self.workflow.as_ref(),
        );
        ProcessStatus::Normal
    }
}

impl ClapPlugin for TranslatePlugin {
    const CLAP_ID: &'static str = "com.placeholdervendor.translate";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Mix translation checking plugin with bundled IR presets and workflow tools");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Utility,
        ClapFeature::Mixing,
        ClapFeature::Mastering,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for TranslatePlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"TranslatePlugin!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
