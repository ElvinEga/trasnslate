mod convolver;

use crate::ir::StereoIr;
use crate::params::TranslateParams;
use convolver::StereoConvolver;
use nih_plug::prelude::Buffer;

#[derive(Debug, Default)]
pub struct TranslateProcessor {
    sample_rate: f32,
    ir_sample_rate: f32,
    convolver: StereoConvolver,
}

impl TranslateProcessor {
    pub fn prepare(&mut self, sample_rate: f32, ir: &StereoIr) {
        self.sample_rate = sample_rate;
        self.ir_sample_rate = ir.sample_rate as f32;
        self.convolver.load_ir(ir);
        self.reset();
    }

    pub fn reset(&mut self) {
        let _ = (self.sample_rate, self.ir_sample_rate);
        self.convolver.reset();
    }

    pub fn process(&mut self, buffer: &mut Buffer, params: &TranslateParams) {
        if params.bypass.value() {
            self.convolver.reset();
            return;
        }

        let channels = buffer.as_slice();
        if channels.is_empty() {
            return;
        }

        let is_mono = params.mono.value();

        match channels {
            [] => {}
            [left] => {
                for sample in left.iter_mut() {
                    let input = *sample;
                    let wet = self.convolver.process_mono_sample(input);
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

                    let [wet_left, wet_right] = self
                        .convolver
                        .process_stereo_sample(input_left, input_right);
                    let mix = params.mix.smoothed.next();
                    let output_gain = params.output.smoothed.next();

                    left[index] = ((1.0 - mix) * input_left + mix * wet_left) * output_gain;
                    right[index] = ((1.0 - mix) * input_right + mix * wet_right) * output_gain;
                }
            }
        }
    }
}
