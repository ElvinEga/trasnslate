use crate::params::{PresetId, QuickCycleMode, TranslateParams};
use nih_plug::prelude::{Enum, Param};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

const NONE_INDEX: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareSlotId {
    None,
    A,
    B,
}

#[derive(Debug, Clone, Copy)]
pub struct ParameterSnapshot {
    pub preset: PresetId,
    pub decay: f32,
    pub mix: f32,
    pub width: f32,
    pub low: f32,
    pub high: f32,
    pub output: f32,
    pub mono: bool,
    pub bypass: bool,
    pub quick_cycle_mode: QuickCycleMode,
    pub quick_cycle_switch_time_ms: i32,
    pub quick_cycle_crossfade_ms: i32,
    pub loudness_lock: bool,
    pub quick_cycle_return_to_reference: bool,
    pub safety_limiter: bool,
}

impl ParameterSnapshot {
    pub fn from_params(params: &TranslateParams) -> Self {
        Self {
            preset: params.preset.unmodulated_plain_value(),
            decay: params.decay.unmodulated_plain_value(),
            mix: params.mix.unmodulated_plain_value(),
            width: params.width.unmodulated_plain_value(),
            low: params.low.unmodulated_plain_value(),
            high: params.high.unmodulated_plain_value(),
            output: params.output.unmodulated_plain_value(),
            mono: params.mono.unmodulated_plain_value(),
            bypass: params.bypass.unmodulated_plain_value(),
            quick_cycle_mode: params.quick_cycle_mode.unmodulated_plain_value(),
            quick_cycle_switch_time_ms: params.quick_cycle_switch_time_ms.unmodulated_plain_value(),
            quick_cycle_crossfade_ms: params.quick_cycle_crossfade_ms.unmodulated_plain_value(),
            loudness_lock: params.quick_cycle_loudness_lock.unmodulated_plain_value(),
            quick_cycle_return_to_reference: params
                .quick_cycle_return_to_reference
                .unmodulated_plain_value(),
            safety_limiter: params.safety_limiter.unmodulated_plain_value(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WorkflowSnapshot {
    pub input_peak: f32,
    pub output_peak: f32,
    pub sample_rate: f32,
    pub latency_samples: u32,
    pub ir_samples: u32,
    pub active_preset: Option<PresetId>,
    pub loudness_compensation_gain: f32,
    pub limiter_gain_reduction_db: f32,
    pub bypass_active: bool,
    pub active_compare_slot: CompareSlotId,
    pub has_a_snapshot: bool,
    pub has_b_snapshot: bool,
}

#[derive(Debug, Default)]
struct CompareState {
    a: Option<ParameterSnapshot>,
    b: Option<ParameterSnapshot>,
}

#[derive(Debug)]
pub struct WorkflowShared {
    input_peak: AtomicU32,
    output_peak: AtomicU32,
    sample_rate: AtomicU32,
    latency_samples: AtomicU32,
    ir_samples: AtomicU32,
    active_preset: AtomicU32,
    loudness_compensation_gain: AtomicU32,
    limiter_gain_reduction_db: AtomicU32,
    bypass_active: AtomicBool,
    active_compare_slot: AtomicU32,
    compare_state: Mutex<CompareState>,
}

impl Default for WorkflowShared {
    fn default() -> Self {
        Self {
            input_peak: AtomicU32::new(0.0f32.to_bits()),
            output_peak: AtomicU32::new(0.0f32.to_bits()),
            sample_rate: AtomicU32::new(44_100.0f32.to_bits()),
            latency_samples: AtomicU32::new(0),
            ir_samples: AtomicU32::new(0),
            active_preset: AtomicU32::new(NONE_INDEX),
            loudness_compensation_gain: AtomicU32::new(1.0f32.to_bits()),
            limiter_gain_reduction_db: AtomicU32::new(0.0f32.to_bits()),
            bypass_active: AtomicBool::new(false),
            active_compare_slot: AtomicU32::new(CompareSlotId::None.code()),
            compare_state: Mutex::new(CompareState::default()),
        }
    }
}

impl WorkflowShared {
    pub fn store_a(&self, snapshot: ParameterSnapshot) {
        let mut state = self
            .compare_state
            .lock()
            .expect("compare state should not be poisoned");
        state.a = Some(snapshot);
    }

    pub fn store_b(&self, snapshot: ParameterSnapshot) {
        let mut state = self
            .compare_state
            .lock()
            .expect("compare state should not be poisoned");
        state.b = Some(snapshot);
    }

    pub fn recall_a(&self) -> Option<ParameterSnapshot> {
        self.compare_state
            .lock()
            .expect("compare state should not be poisoned")
            .a
    }

    pub fn recall_b(&self) -> Option<ParameterSnapshot> {
        self.compare_state
            .lock()
            .expect("compare state should not be poisoned")
            .b
    }

    pub fn set_active_compare_slot(&self, slot: CompareSlotId) {
        self.active_compare_slot
            .store(slot.code(), Ordering::Relaxed);
    }

    pub fn update_status(
        &self,
        sample_rate: f32,
        latency_samples: u32,
        ir_samples: u32,
        active_preset: Option<PresetId>,
        loudness_compensation_gain: f32,
        limiter_gain_reduction_db: f32,
        bypass_active: bool,
    ) {
        self.sample_rate
            .store(sample_rate.to_bits(), Ordering::Relaxed);
        self.latency_samples
            .store(latency_samples, Ordering::Relaxed);
        self.ir_samples.store(ir_samples, Ordering::Relaxed);
        self.active_preset.store(
            active_preset.map_or(NONE_INDEX, |preset| preset.to_index() as u32),
            Ordering::Relaxed,
        );
        self.loudness_compensation_gain
            .store(loudness_compensation_gain.to_bits(), Ordering::Relaxed);
        self.limiter_gain_reduction_db
            .store(limiter_gain_reduction_db.to_bits(), Ordering::Relaxed);
        self.bypass_active.store(bypass_active, Ordering::Relaxed);
    }

    pub fn update_meters(&self, input_peak: f32, output_peak: f32) {
        self.input_peak
            .store(input_peak.to_bits(), Ordering::Relaxed);
        self.output_peak
            .store(output_peak.to_bits(), Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> WorkflowSnapshot {
        let state = self
            .compare_state
            .lock()
            .expect("compare state should not be poisoned");

        WorkflowSnapshot {
            input_peak: f32::from_bits(self.input_peak.load(Ordering::Relaxed)),
            output_peak: f32::from_bits(self.output_peak.load(Ordering::Relaxed)),
            sample_rate: f32::from_bits(self.sample_rate.load(Ordering::Relaxed)),
            latency_samples: self.latency_samples.load(Ordering::Relaxed),
            ir_samples: self.ir_samples.load(Ordering::Relaxed),
            active_preset: preset_from_u32(self.active_preset.load(Ordering::Relaxed)),
            loudness_compensation_gain: f32::from_bits(
                self.loudness_compensation_gain.load(Ordering::Relaxed),
            ),
            limiter_gain_reduction_db: f32::from_bits(
                self.limiter_gain_reduction_db.load(Ordering::Relaxed),
            ),
            bypass_active: self.bypass_active.load(Ordering::Relaxed),
            active_compare_slot: CompareSlotId::from_code(
                self.active_compare_slot.load(Ordering::Relaxed),
            ),
            has_a_snapshot: state.a.is_some(),
            has_b_snapshot: state.b.is_some(),
        }
    }
}

impl CompareSlotId {
    fn code(self) -> u32 {
        match self {
            Self::None => 0,
            Self::A => 1,
            Self::B => 2,
        }
    }

    fn from_code(code: u32) -> Self {
        match code {
            1 => Self::A,
            2 => Self::B,
            _ => Self::None,
        }
    }
}

fn preset_from_u32(value: u32) -> Option<PresetId> {
    if value == NONE_INDEX {
        None
    } else {
        Some(PresetId::from_index(value as usize))
    }
}
