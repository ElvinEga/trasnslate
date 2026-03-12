use crate::ir::StereoIr;

#[derive(Debug, Default)]
pub struct StereoConvolver {
    ir_left: Vec<f32>,
    ir_right: Vec<f32>,
    history_left: Vec<f32>,
    history_right: Vec<f32>,
    write_index: usize,
}

impl StereoConvolver {
    pub fn load_ir(&mut self, ir: &StereoIr) {
        self.ir_left.clear();
        self.ir_left.extend_from_slice(&ir.left);
        self.ir_right.clear();
        self.ir_right.extend_from_slice(&ir.right);

        self.history_left.resize(ir.len(), 0.0);
        self.history_right.resize(ir.len(), 0.0);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.history_left.fill(0.0);
        self.history_right.fill(0.0);
        self.write_index = 0;
    }

    pub fn process_mono_sample(&mut self, input: f32) -> f32 {
        self.process_stereo_sample(input, input)[0]
    }

    pub fn process_stereo_sample(&mut self, input_left: f32, input_right: f32) -> [f32; 2] {
        if self.ir_left.is_empty() {
            return [input_left, input_right];
        }

        self.history_left[self.write_index] = input_left;
        self.history_right[self.write_index] = input_right;

        let wet_left = convolve_channel(&self.ir_left, &self.history_left, self.write_index);
        let wet_right = convolve_channel(&self.ir_right, &self.history_right, self.write_index);

        self.write_index += 1;
        if self.write_index == self.ir_left.len() {
            self.write_index = 0;
        }

        [wet_left, wet_right]
    }
}

fn convolve_channel(ir: &[f32], history: &[f32], write_index: usize) -> f32 {
    let mut output = 0.0;
    let mut history_index = write_index;

    for &tap in ir {
        output += tap * history[history_index];
        history_index = if history_index == 0 {
            history.len() - 1
        } else {
            history_index - 1
        };
    }

    output
}
