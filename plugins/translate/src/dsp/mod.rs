use nih_plug::prelude::Buffer;

#[derive(Debug, Default)]
pub struct TranslateProcessor {
    sample_rate: f32,
}

impl TranslateProcessor {
    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn reset(&mut self) {
        let _ = self.sample_rate;
    }

    pub fn process(&mut self, _buffer: &mut Buffer) {
        // The host-provided input has already been copied to the output by NIH-plug.
        // Milestone 1 intentionally leaves the signal untouched.
    }
}
