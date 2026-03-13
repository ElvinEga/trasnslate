use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetId {
    #[id = "car-hatchback"]
    #[name = "Hatchback"]
    CarHatchback,
    #[id = "phone-speaker"]
    #[name = "Phone Speaker"]
    PhoneSpeaker,
    #[id = "tablet-laptop"]
    #[name = "Tablet / Laptop"]
    TabletLaptop,
    #[id = "club-booth"]
    #[name = "Club Booth"]
    ClubBooth,
    #[id = "concert-venue"]
    #[name = "Concert Venue"]
    ConcertVenue,
    #[id = "mono-radio"]
    #[name = "Mono Radio"]
    MonoRadio,
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickCycleMode {
    Manual,
    Timed,
}

#[derive(Params)]
pub struct TranslateParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "preset"]
    pub preset: EnumParam<PresetId>,
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
    #[id = "qc-mode"]
    pub quick_cycle_mode: EnumParam<QuickCycleMode>,
    #[id = "qc-switch-ms"]
    pub quick_cycle_switch_time_ms: IntParam,
    #[id = "qc-fade-ms"]
    pub quick_cycle_crossfade_ms: IntParam,
    #[id = "qc-lock"]
    pub quick_cycle_loudness_lock: BoolParam,
    #[id = "qc-return-ref"]
    pub quick_cycle_return_to_reference: BoolParam,
}

impl Default for TranslateParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(640, 520),
            preset: EnumParam::new("Preset", PresetId::CarHatchback),
            decay: FloatParam::new("Decay", 1.0, FloatRange::Linear { min: 0.1, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(50.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(1))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            width: FloatParam::new("Width", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_value_to_string(Arc::new(|value| format!("{value:.2}x"))),
            low: FloatParam::new(
                "Low",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit(" dB"),
            high: FloatParam::new(
                "High",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
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
            quick_cycle_mode: EnumParam::new("Quick Cycle Mode", QuickCycleMode::Manual),
            quick_cycle_switch_time_ms: IntParam::new(
                "Switch Time",
                2500,
                IntRange::Linear {
                    min: 250,
                    max: 10_000,
                },
            )
            .with_unit(" ms"),
            quick_cycle_crossfade_ms: IntParam::new(
                "Crossfade Time",
                30,
                IntRange::Linear { min: 5, max: 200 },
            )
            .with_unit(" ms"),
            quick_cycle_loudness_lock: BoolParam::new("Loudness Lock", false),
            quick_cycle_return_to_reference: BoolParam::new("Return to Reference", true),
        }
    }
}
