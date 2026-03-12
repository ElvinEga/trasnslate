use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetMode {
    #[id = "flat"]
    Flat,
    #[id = "nearfield"]
    Nearfield,
    #[id = "small-speaker"]
    #[name = "Small Speaker"]
    SmallSpeaker,
    #[id = "consumer"]
    Consumer,
}

#[derive(Params)]
pub struct TranslateParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "preset"]
    pub preset: EnumParam<PresetMode>,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "width"]
    pub width: FloatParam,
    #[id = "low"]
    pub low: FloatParam,
    #[id = "high"]
    pub high: FloatParam,
    #[id = "output"]
    pub output: FloatParam,
    #[id = "mono"]
    pub mono: BoolParam,
    #[id = "bypass"]
    pub bypass: BoolParam,
    #[id = "quick-cycle"]
    pub quick_cycle: BoolParam,
}

impl Default for TranslateParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(420, 320),
            preset: EnumParam::new("Preset", PresetMode::Flat),
            decay: FloatParam::new("Decay", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(1))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            width: FloatParam::new("Width", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 }),
            low: FloatParam::new(
                "Low",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_unit(" dB"),
            high: FloatParam::new(
                "High",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_unit(" dB"),
            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(24.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 24.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(20.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            mono: BoolParam::new("Mono", false),
            bypass: BoolParam::new("Bypass", false),
            quick_cycle: BoolParam::new("Quick Cycle", false),
        }
    }
}
