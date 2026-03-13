mod convolver;
mod tone;

use crate::ir::IrState;
use crate::params::{PresetId, QuickCycleMode, TranslateParams};
use crate::quick_cycle::{QuickCycleAction, QuickCycleShared, QuickCycleSnapshot};
use convolver::StereoConvolver;
use nih_plug::prelude::Buffer;
use tone::StereoToneStack;

#[derive(Debug)]
pub struct TranslateProcessor {
    sample_rate: f32,
    current_preset: PresetId,
    current_decay: f32,
    pending_preset: Option<PresetId>,
    pending_decay: f32,
    current_convolver: StereoConvolver,
    next_convolver: StereoConvolver,
    tone: StereoToneStack,
    preset_crossfade_samples: usize,
    preset_crossfade_position: usize,
    work_left: Vec<f32>,
    work_right: Vec<f32>,
    cycle_active: bool,
    cycle_running: bool,
    cycle_current_slot: Option<usize>,
    cycle_reference_preset: PresetId,
    samples_until_cycle_step: usize,
}

impl TranslateProcessor {
    pub fn prepare(
        &mut self,
        sample_rate: f32,
        ir_state: &IrState,
        params: &TranslateParams,
        quick_cycle: &QuickCycleShared,
    ) {
        self.sample_rate = sample_rate.max(1.0);

        let max_ir_len = ir_state.max_prepared_len();
        self.current_convolver.prepare(max_ir_len);
        self.next_convolver.prepare(max_ir_len);
        self.work_left.resize(max_ir_len, 0.0);
        self.work_right.resize(max_ir_len, 0.0);

        self.current_preset = params.preset.value();
        self.current_decay = params.decay.value();
        self.pending_preset = None;
        self.preset_crossfade_position = self.preset_crossfade_samples.max(1);
        self.cycle_active = false;
        self.cycle_running = false;
        self.cycle_current_slot = None;
        self.cycle_reference_preset = self.current_preset;
        self.samples_until_cycle_step = self.switch_time_samples(params);

        let len = self.shape_into_work_buffers(ir_state, self.current_preset, self.current_decay);
        self.current_convolver
            .load_ir(&self.work_left[..len], &self.work_right[..len]);
        self.next_convolver.reset();
        self.tone.reset();
        quick_cycle.set_status(Some(self.current_preset), None, false);
    }

    pub fn reset(&mut self, quick_cycle: &QuickCycleShared) {
        self.current_convolver.reset();
        self.next_convolver.reset();
        self.tone.reset();
        self.preset_crossfade_position = self.preset_crossfade_samples.max(1);
        self.cycle_running = false;
        quick_cycle.set_status(Some(self.current_preset), None, false);
    }

    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        params: &TranslateParams,
        ir_state: &IrState,
        quick_cycle: &QuickCycleShared,
    ) {
        if params.bypass.value() {
            self.reset(quick_cycle);
            return;
        }

        let snapshot = quick_cycle.snapshot();
        self.handle_cycle_action(
            quick_cycle.take_action(),
            &snapshot,
            params,
            params.preset.value(),
        );
        self.handle_timed_cycle(&snapshot, params, params.preset.value(), buffer.samples());

        let target_preset = self.effective_preset(params.preset.value(), &snapshot);
        self.sync_ir_target(
            ir_state,
            params.quick_cycle_crossfade_ms.value() as usize,
            target_preset,
            params.decay.value(),
        );

        let next_cycle_preset = self.next_cycle_preset(&snapshot, params.preset.value());
        quick_cycle.set_status(
            Some(target_preset),
            next_cycle_preset,
            self.cycle_running && params.quick_cycle_mode.value() == QuickCycleMode::Timed,
        );

        let channels = buffer.as_slice();
        if channels.is_empty() {
            return;
        }

        let is_mono = params.mono.value();

        match channels {
            [] => {}
            [mono] => {
                for sample in mono.iter_mut() {
                    let input = *sample;
                    let [wet, _] = self.process_wet_pair(input, input, params, false);
                    let mix = params.mix.smoothed.next();
                    let output_gain = params.output.smoothed.next();
                    *sample = ((1.0 - mix) * input + mix * wet) * output_gain;
                }
            }
            [left, right, ..] => {
                for index in 0..left.len() {
                    let dry_left = left[index];
                    let dry_right = right[index];
                    let (input_left, input_right) = if is_mono {
                        let mono = 0.5 * (dry_left + dry_right);
                        (mono, mono)
                    } else {
                        (dry_left, dry_right)
                    };

                    let [wet_left, wet_right] =
                        self.process_wet_pair(input_left, input_right, params, true);
                    let mix = params.mix.smoothed.next();
                    let output_gain = params.output.smoothed.next();

                    left[index] = ((1.0 - mix) * input_left + mix * wet_left) * output_gain;
                    right[index] = ((1.0 - mix) * input_right + mix * wet_right) * output_gain;
                }
            }
        }
    }

    fn process_wet_pair(
        &mut self,
        input_left: f32,
        input_right: f32,
        params: &TranslateParams,
        apply_width: bool,
    ) -> [f32; 2] {
        let current_wet = self
            .current_convolver
            .process_stereo_sample(input_left, input_right);

        let wet = if let Some(pending_preset) = self.pending_preset {
            let pending_wet = self
                .next_convolver
                .process_stereo_sample(input_left, input_right);
            let fade_t =
                self.preset_crossfade_position as f32 / self.preset_crossfade_samples as f32;
            self.preset_crossfade_position += 1;

            if self.preset_crossfade_position >= self.preset_crossfade_samples {
                std::mem::swap(&mut self.current_convolver, &mut self.next_convolver);
                self.next_convolver.reset();
                self.current_preset = pending_preset;
                self.current_decay = self.pending_decay;
                self.pending_preset = None;
                self.preset_crossfade_position = self.preset_crossfade_samples;
            }

            [
                current_wet[0] * (1.0 - fade_t) + pending_wet[0] * fade_t,
                current_wet[1] * (1.0 - fade_t) + pending_wet[1] * fade_t,
            ]
        } else {
            current_wet
        };

        let low = params.low.smoothed.next();
        let high = params.high.smoothed.next();
        self.tone.update(self.sample_rate, low, high);
        let [mut wet_left, mut wet_right] = self.tone.process(wet[0], wet[1]);

        if apply_width {
            let width = params.width.smoothed.next();
            let mid = 0.5 * (wet_left + wet_right);
            let side = 0.5 * (wet_left - wet_right) * width;
            wet_left = mid + side;
            wet_right = mid - side;
        }

        [wet_left, wet_right]
    }

    fn handle_cycle_action(
        &mut self,
        action: QuickCycleAction,
        snapshot: &QuickCycleSnapshot,
        params: &TranslateParams,
        base_preset: PresetId,
    ) {
        match action {
            QuickCycleAction::None => {}
            QuickCycleAction::Previous => self.step_cycle(snapshot, base_preset, false),
            QuickCycleAction::Next => self.step_cycle(snapshot, base_preset, true),
            QuickCycleAction::StartOrCycle => match params.quick_cycle_mode.value() {
                QuickCycleMode::Manual => self.step_cycle(snapshot, base_preset, true),
                QuickCycleMode::Timed => self.start_timed_cycle(snapshot, base_preset, params),
            },
            QuickCycleAction::PauseOrStop => self.pause_or_stop_cycle(params),
            QuickCycleAction::ReturnToReference => self.return_to_reference(),
        }
    }

    fn handle_timed_cycle(
        &mut self,
        snapshot: &QuickCycleSnapshot,
        params: &TranslateParams,
        base_preset: PresetId,
        num_samples: usize,
    ) {
        if !self.cycle_running || params.quick_cycle_mode.value() != QuickCycleMode::Timed {
            return;
        }

        if self.samples_until_cycle_step <= num_samples {
            self.step_cycle(snapshot, base_preset, true);
            self.samples_until_cycle_step = self.switch_time_samples(params);
        } else {
            self.samples_until_cycle_step -= num_samples;
        }
    }

    fn step_cycle(&mut self, snapshot: &QuickCycleSnapshot, base_preset: PresetId, forward: bool) {
        if let Some(next_slot) = next_enabled_slot(
            snapshot,
            self.cycle_current_slot,
            forward,
            self.cycle_active.then_some(base_preset),
        ) {
            if !self.cycle_active {
                self.cycle_reference_preset = base_preset;
                self.cycle_active = true;
            }
            self.cycle_current_slot = Some(next_slot);
        }
    }

    fn start_timed_cycle(
        &mut self,
        snapshot: &QuickCycleSnapshot,
        base_preset: PresetId,
        params: &TranslateParams,
    ) {
        if !self.cycle_active {
            self.step_cycle(snapshot, base_preset, true);
        }

        if self.cycle_active {
            self.cycle_running = true;
            self.samples_until_cycle_step = self.switch_time_samples(params);
        }
    }

    fn pause_or_stop_cycle(&mut self, params: &TranslateParams) {
        self.cycle_running = false;
        if params.quick_cycle_mode.value() == QuickCycleMode::Manual
            && params.quick_cycle_return_to_reference.value()
        {
            self.return_to_reference();
        } else if params.quick_cycle_mode.value() == QuickCycleMode::Timed
            && params.quick_cycle_return_to_reference.value()
        {
            self.return_to_reference();
        }
    }

    fn return_to_reference(&mut self) {
        self.cycle_active = false;
        self.cycle_running = false;
        self.cycle_current_slot = None;
    }

    fn effective_preset(&self, base_preset: PresetId, snapshot: &QuickCycleSnapshot) -> PresetId {
        if let Some(slot) = self.cycle_current_slot {
            snapshot.slots[slot].preset
        } else {
            base_preset
        }
    }

    fn next_cycle_preset(
        &self,
        snapshot: &QuickCycleSnapshot,
        base_preset: PresetId,
    ) -> Option<PresetId> {
        if let Some(slot) = self.cycle_current_slot {
            next_enabled_slot(snapshot, Some(slot), true, None)
                .map(|next| snapshot.slots[next].preset)
        } else {
            next_enabled_slot(snapshot, None, true, Some(base_preset))
                .map(|next| snapshot.slots[next].preset)
        }
    }

    fn sync_ir_target(
        &mut self,
        ir_state: &IrState,
        crossfade_ms: usize,
        preset: PresetId,
        decay: f32,
    ) {
        let queued_preset = self.pending_preset.unwrap_or(self.current_preset);
        let queued_decay = if self.pending_preset.is_some() {
            self.pending_decay
        } else {
            self.current_decay
        };

        if queued_preset == preset && (queued_decay - decay).abs() < 1.0e-3 {
            return;
        }

        self.preset_crossfade_samples =
            ((self.sample_rate * crossfade_ms.max(1) as f32 / 1000.0).round() as usize).max(1);

        let len = self.shape_into_work_buffers(ir_state, preset, decay);
        self.next_convolver
            .load_ir(&self.work_left[..len], &self.work_right[..len]);
        self.pending_preset = Some(preset);
        self.pending_decay = decay;
        self.preset_crossfade_position = 0;
    }

    fn shape_into_work_buffers(
        &mut self,
        ir_state: &IrState,
        preset: PresetId,
        decay: f32,
    ) -> usize {
        let prepared = ir_state.prepared_preset(preset);
        let len = prepared.ir.len();
        shape_ir(
            &prepared.ir.left[..len],
            decay,
            &mut self.work_left[..len],
            len,
        );
        shape_ir(
            &prepared.ir.right[..len],
            decay,
            &mut self.work_right[..len],
            len,
        );
        len
    }

    fn switch_time_samples(&self, params: &TranslateParams) -> usize {
        ((self.sample_rate * params.quick_cycle_switch_time_ms.value() as f32 / 1000.0).round()
            as usize)
            .max(1)
    }
}

impl Default for TranslateProcessor {
    fn default() -> Self {
        Self {
            sample_rate: 44_100.0,
            current_preset: PresetId::CarHatchback,
            current_decay: 1.0,
            pending_preset: None,
            pending_decay: 1.0,
            current_convolver: StereoConvolver::default(),
            next_convolver: StereoConvolver::default(),
            tone: StereoToneStack::default(),
            preset_crossfade_samples: 1,
            preset_crossfade_position: 1,
            work_left: Vec::new(),
            work_right: Vec::new(),
            cycle_active: false,
            cycle_running: false,
            cycle_current_slot: None,
            cycle_reference_preset: PresetId::CarHatchback,
            samples_until_cycle_step: 1,
        }
    }
}

fn shape_ir(source: &[f32], decay: f32, out: &mut [f32], len: usize) {
    let decay = decay.clamp(0.1, 1.0);
    let exponent = (1.0 - decay) * 6.0;

    if len <= 1 {
        out[0] = source[0];
        return;
    }

    for index in 0..len {
        let position = index as f32 / (len - 1) as f32;
        let tail = if exponent <= 0.0 {
            1.0
        } else {
            (1.0 - position).powf(exponent)
        };
        out[index] = source[index] * tail;
    }
}

fn next_enabled_slot(
    snapshot: &QuickCycleSnapshot,
    current_slot: Option<usize>,
    forward: bool,
    _base_preset: Option<PresetId>,
) -> Option<usize> {
    if snapshot.slots.iter().all(|slot| !slot.enabled) {
        return None;
    }

    let len = snapshot.slots.len();
    let start = current_slot.unwrap_or(if forward { len - 1 } else { 0 });

    for offset in 1..=len {
        let slot = if forward {
            (start + offset) % len
        } else {
            (start + len - offset) % len
        };

        if snapshot.slots[slot].enabled {
            return Some(slot);
        }
    }

    None
}
