mod convolver;
mod tone;

use crate::ir::IrState;
use crate::params::{PresetId, QuickCycleMode, TranslateParams};
use crate::quick_cycle::{QuickCycleAction, QuickCycleShared, QuickCycleSnapshot};
use crate::workflow::WorkflowShared;
use convolver::StereoConvolver;
use nih_plug::prelude::Buffer;
use tone::StereoToneStack;

const BYPASS_SMOOTH_MS: f32 = 12.0;
const LOUDNESS_ATTACK_MS: f32 = 80.0;
const LOUDNESS_RELEASE_MS: f32 = 350.0;
const LIMITER_ATTACK_MS: f32 = 1.5;
const LIMITER_RELEASE_MS: f32 = 120.0;
const LIMITER_THRESHOLD: f32 = 0.98;

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
    samples_until_cycle_step: usize,
    bypass_mix: f32,
    input_meter_peak: f32,
    output_meter_peak: f32,
    loudness_input_power: f32,
    loudness_output_power: f32,
    loudness_gain: f32,
    limiter_gain: f32,
    limiter_gain_reduction_db: f32,
    active_ir_len: usize,
}

impl TranslateProcessor {
    pub fn prepare(
        &mut self,
        sample_rate: f32,
        ir_state: &IrState,
        params: &TranslateParams,
        quick_cycle: &QuickCycleShared,
        workflow: &WorkflowShared,
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
        self.samples_until_cycle_step = self.switch_time_samples(params);
        self.bypass_mix = if params.bypass.value() { 0.0 } else { 1.0 };
        self.input_meter_peak = 0.0;
        self.output_meter_peak = 0.0;
        self.loudness_input_power = 1.0e-6;
        self.loudness_output_power = 1.0e-6;
        self.loudness_gain = 1.0;
        self.limiter_gain = 1.0;
        self.limiter_gain_reduction_db = 0.0;

        let len = self.shape_into_work_buffers(ir_state, self.current_preset, self.current_decay);
        self.active_ir_len = len;
        self.current_convolver
            .load_ir(&self.work_left[..len], &self.work_right[..len]);
        self.next_convolver.reset();
        self.tone.reset();

        quick_cycle.set_status(Some(self.current_preset), None, false);
        workflow.update_meters(0.0, 0.0);
        workflow.update_status(
            self.sample_rate,
            0,
            self.active_ir_len as u32,
            Some(self.current_preset),
            self.loudness_gain,
            self.limiter_gain_reduction_db,
            params.bypass.value(),
        );
    }

    pub fn reset(&mut self, quick_cycle: &QuickCycleShared, workflow: &WorkflowShared) {
        self.current_convolver.reset();
        self.next_convolver.reset();
        self.tone.reset();
        self.preset_crossfade_position = self.preset_crossfade_samples.max(1);
        self.cycle_running = false;
        self.input_meter_peak = 0.0;
        self.output_meter_peak = 0.0;
        self.loudness_input_power = 1.0e-6;
        self.loudness_output_power = 1.0e-6;
        self.loudness_gain = 1.0;
        self.limiter_gain = 1.0;
        self.limiter_gain_reduction_db = 0.0;

        quick_cycle.set_status(Some(self.current_preset), None, false);
        workflow.update_meters(0.0, 0.0);
        workflow.update_status(
            self.sample_rate,
            0,
            self.active_ir_len as u32,
            Some(self.current_preset),
            self.loudness_gain,
            self.limiter_gain_reduction_db,
            false,
        );
    }

    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        params: &TranslateParams,
        ir_state: &IrState,
        quick_cycle: &QuickCycleShared,
        workflow: &WorkflowShared,
    ) {
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
            workflow.update_meters(0.0, 0.0);
            workflow.update_status(
                self.sample_rate,
                0,
                self.active_ir_len as u32,
                Some(target_preset),
                self.loudness_gain,
                self.limiter_gain_reduction_db,
                params.bypass.value(),
            );
            return;
        }

        let is_mono = params.mono.value();
        let bypass_target = if params.bypass.value() { 0.0 } else { 1.0 };
        let bypass_coeff = smoothing_coeff(self.sample_rate, BYPASS_SMOOTH_MS);
        let loudness_enabled = params.quick_cycle_loudness_lock.value();
        let limiter_enabled = params.safety_limiter.value();
        let mut block_input_peak: f32 = 0.0;
        let mut block_output_peak: f32 = 0.0;

        match channels {
            [] => {}
            [mono] => {
                for sample in mono.iter_mut() {
                    let dry = *sample;
                    block_input_peak = block_input_peak.max(dry.abs());

                    let [wet, _] = self.process_wet_pair(dry, dry, params, false);
                    let mix = params.mix.smoothed.next();
                    let output_gain = params.output.smoothed.next();
                    let processed = (1.0 - mix) * dry + mix * wet;
                    let [processed, _] =
                        self.apply_loudness_lock(processed, processed, dry, dry, loudness_enabled);
                    let mut out = processed * output_gain;
                    let [limited, _, gain_reduction_db] =
                        self.apply_safety_limiter(out, out, limiter_enabled);
                    out = limited;
                    self.limiter_gain_reduction_db = gain_reduction_db;

                    self.bypass_mix += (bypass_target - self.bypass_mix) * bypass_coeff;
                    let final_out = dry * (1.0 - self.bypass_mix) + out * self.bypass_mix;
                    *sample = final_out;
                    block_output_peak = block_output_peak.max(final_out.abs());
                }
            }
            [left, right, ..] => {
                for index in 0..left.len() {
                    let dry_left = left[index];
                    let dry_right = right[index];
                    block_input_peak = block_input_peak.max(dry_left.abs().max(dry_right.abs()));

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
                    let processed_left = (1.0 - mix) * input_left + mix * wet_left;
                    let processed_right = (1.0 - mix) * input_right + mix * wet_right;
                    let [comp_left, comp_right] = self.apply_loudness_lock(
                        processed_left,
                        processed_right,
                        input_left,
                        input_right,
                        loudness_enabled,
                    );
                    let out_left = comp_left * output_gain;
                    let out_right = comp_right * output_gain;
                    let [limited_left, limited_right, gain_reduction_db] =
                        self.apply_safety_limiter(out_left, out_right, limiter_enabled);
                    self.limiter_gain_reduction_db = gain_reduction_db;

                    self.bypass_mix += (bypass_target - self.bypass_mix) * bypass_coeff;
                    let final_left =
                        dry_left * (1.0 - self.bypass_mix) + limited_left * self.bypass_mix;
                    let final_right =
                        dry_right * (1.0 - self.bypass_mix) + limited_right * self.bypass_mix;

                    left[index] = final_left;
                    right[index] = final_right;
                    block_output_peak =
                        block_output_peak.max(final_left.abs().max(final_right.abs()));
                }
            }
        }

        self.input_meter_peak = (self.input_meter_peak * 0.9).max(block_input_peak);
        self.output_meter_peak = (self.output_meter_peak * 0.9).max(block_output_peak);
        workflow.update_meters(self.input_meter_peak, self.output_meter_peak);
        workflow.update_status(
            self.sample_rate,
            0,
            self.active_ir_len as u32,
            Some(target_preset),
            self.loudness_gain,
            self.limiter_gain_reduction_db,
            params.bypass.value(),
        );
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

    fn apply_loudness_lock(
        &mut self,
        left: f32,
        right: f32,
        ref_left: f32,
        ref_right: f32,
        enabled: bool,
    ) -> [f32; 2] {
        let input_power = 0.5 * (ref_left * ref_left + ref_right * ref_right);
        let output_power = 0.5 * (left * left + right * right);
        let attack = smoothing_coeff(self.sample_rate, LOUDNESS_ATTACK_MS);
        let release = smoothing_coeff(self.sample_rate, LOUDNESS_RELEASE_MS);

        self.loudness_input_power += (input_power - self.loudness_input_power) * attack;
        self.loudness_output_power += (output_power - self.loudness_output_power) * attack;

        let target_gain = if enabled && self.loudness_input_power > 1.0e-6 {
            (self.loudness_input_power / self.loudness_output_power.max(1.0e-6))
                .sqrt()
                .clamp(0.25, 4.0)
        } else {
            1.0
        };
        let coeff = if target_gain < self.loudness_gain {
            attack
        } else {
            release
        };
        self.loudness_gain += (target_gain - self.loudness_gain) * coeff;

        [left * self.loudness_gain, right * self.loudness_gain]
    }

    fn apply_safety_limiter(&mut self, left: f32, right: f32, enabled: bool) -> [f32; 3] {
        if !enabled {
            self.limiter_gain +=
                (1.0 - self.limiter_gain) * smoothing_coeff(self.sample_rate, LIMITER_RELEASE_MS);
            self.limiter_gain_reduction_db = 0.0;
            return [left, right, 0.0];
        }

        let peak = left.abs().max(right.abs());
        let desired_gain = if peak > LIMITER_THRESHOLD {
            (LIMITER_THRESHOLD / peak).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let coeff = if desired_gain < self.limiter_gain {
            smoothing_coeff(self.sample_rate, LIMITER_ATTACK_MS)
        } else {
            smoothing_coeff(self.sample_rate, LIMITER_RELEASE_MS)
        };
        self.limiter_gain += (desired_gain - self.limiter_gain) * coeff;

        let gain_reduction_db = if self.limiter_gain < 0.999 {
            20.0 * self.limiter_gain.max(1.0e-6).log10().abs()
        } else {
            0.0
        };

        [
            left * self.limiter_gain,
            right * self.limiter_gain,
            gain_reduction_db,
        ]
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
        if let Some(next_slot) = next_enabled_slot(snapshot, self.cycle_current_slot, forward) {
            if !self.cycle_active {
                self.cycle_active = true;
            }

            self.cycle_current_slot = Some(next_slot);
            self.current_preset = self.effective_preset(base_preset, snapshot);
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
        if params.quick_cycle_return_to_reference.value() {
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
            next_enabled_slot(snapshot, Some(slot), true).map(|next| snapshot.slots[next].preset)
        } else {
            next_enabled_slot(snapshot, None, true).map(|next| {
                let _ = base_preset;
                snapshot.slots[next].preset
            })
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

        self.active_ir_len = ir_state.prepared_ir_len(preset);

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
            samples_until_cycle_step: 1,
            bypass_mix: 1.0,
            input_meter_peak: 0.0,
            output_meter_peak: 0.0,
            loudness_input_power: 1.0e-6,
            loudness_output_power: 1.0e-6,
            loudness_gain: 1.0,
            limiter_gain: 1.0,
            limiter_gain_reduction_db: 0.0,
            active_ir_len: 0,
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

#[cfg(test)]
mod tests {
    use super::TranslateProcessor;
    use crate::ir::IrState;
    use crate::params::{PresetId, TranslateParams};
    use crate::quick_cycle::QuickCycleShared;
    use crate::workflow::WorkflowShared;

    #[test]
    fn switching_presets_queues_and_activates_a_new_ir() {
        let params = TranslateParams::default();
        let quick_cycle = QuickCycleShared::default();
        let workflow = WorkflowShared::default();
        let mut ir_state = IrState::default();
        ir_state.prepare_for_sample_rate(1_000.0);

        let mut processor = TranslateProcessor::default();
        processor.prepare(1_000.0, &ir_state, &params, &quick_cycle, &workflow);

        let original_len = processor.active_ir_len;
        processor.sync_ir_target(&ir_state, 1, PresetId::Boombox, 1.0);
        assert_eq!(processor.pending_preset, Some(PresetId::Boombox));
        assert_ne!(processor.active_ir_len, 0);

        processor.process_wet_pair(1.0, 1.0, &params, true);

        assert_eq!(processor.pending_preset, None);
        assert_eq!(processor.current_preset, PresetId::Boombox);
        assert_ne!(processor.active_ir_len, original_len);
    }
}

fn smoothing_coeff(sample_rate: f32, time_ms: f32) -> f32 {
    let samples = (sample_rate.max(1.0) * time_ms.max(0.1) / 1000.0).max(1.0);
    1.0 / samples
}
