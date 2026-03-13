#[derive(Debug, Default)]
pub struct StereoConvolver {
    taps_left: Vec<f32>,
    taps_right: Vec<f32>,
    history_left: Vec<f32>,
    history_right: Vec<f32>,
    write_index: usize,
    active_len: usize,
    direct_left: f32,
    direct_right: f32,
}

impl StereoConvolver {
    pub fn prepare(&mut self, max_len: usize) {
        self.taps_left.resize(max_len, 0.0);
        self.taps_right.resize(max_len, 0.0);
        self.history_left.resize(max_len, 0.0);
        self.history_right.resize(max_len, 0.0);
        self.active_len = 0;
        self.write_index = 0;
        self.direct_left = 0.0;
        self.direct_right = 0.0;
    }

    pub fn load_ir(&mut self, left: &[f32], right: &[f32]) {
        let len = left.len().min(right.len());
        debug_assert!(len <= self.taps_left.len());

        self.active_len = len;
        self.direct_left = left.first().copied().unwrap_or(0.0);
        self.direct_right = right.first().copied().unwrap_or(0.0);
        self.taps_left[..len].copy_from_slice(&left[..len]);
        self.taps_right[..len].copy_from_slice(&right[..len]);
        self.taps_left[len..].fill(0.0);
        self.taps_right[len..].fill(0.0);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.history_left.fill(0.0);
        self.history_right.fill(0.0);
        self.write_index = 0;
    }

    pub fn process_stereo_sample(&mut self, input_left: f32, input_right: f32) -> [f32; 2] {
        if self.active_len == 0 {
            return [0.0, 0.0];
        }

        self.history_left[self.write_index] = input_left;
        self.history_right[self.write_index] = input_right;

        let wet_left = convolve_channel(
            &self.taps_left[..self.active_len],
            &self.history_left[..self.active_len],
            self.write_index,
        );
        let wet_right = convolve_channel(
            &self.taps_right[..self.active_len],
            &self.history_right[..self.active_len],
            self.write_index,
        );

        self.write_index += 1;
        if self.write_index == self.active_len {
            self.write_index = 0;
        }

        [
            wet_left - input_left * self.direct_left,
            wet_right - input_right * self.direct_right,
        ]
    }
}

fn convolve_channel(taps: &[f32], history: &[f32], write_index: usize) -> f32 {
    let mut output = 0.0;
    let mut history_index = write_index;

    for &tap in taps {
        output += tap * history[history_index];
        history_index = if history_index == 0 {
            history.len() - 1
        } else {
            history_index - 1
        };
    }

    output
}

#[cfg(test)]
mod tests {
    use super::StereoConvolver;

    #[test]
    fn removes_direct_component_from_wet_output() {
        let mut convolver = StereoConvolver::default();
        convolver.prepare(3);
        convolver.load_ir(&[1.0, 0.5, 0.0], &[1.0, 0.25, 0.0]);

        let first = convolver.process_stereo_sample(1.0, 1.0);
        assert!((first[0] - 0.0).abs() < 1.0e-6);
        assert!((first[1] - 0.0).abs() < 1.0e-6);

        let second = convolver.process_stereo_sample(0.0, 0.0);
        assert!((second[0] - 0.5).abs() < 1.0e-6);
        assert!((second[1] - 0.25).abs() < 1.0e-6);
    }
}
