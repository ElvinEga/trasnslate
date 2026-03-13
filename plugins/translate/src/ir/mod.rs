mod bundled;

pub use bundled::{
    preset_category, preset_name, presets_for_category, BundledIr, PreparedPreset, PresetCategory,
};

use crate::params::PresetId;

#[derive(Debug, Default)]
pub struct IrState {
    raw_presets: Vec<BundledIr>,
    prepared_presets: Vec<PreparedPreset>,
    prepared_sample_rate: Option<u32>,
    max_prepared_len: usize,
}

impl IrState {
    pub fn prepare_for_sample_rate(&mut self, sample_rate: f32) {
        let target_sample_rate = sample_rate.max(1.0).round() as u32;
        if self.prepared_sample_rate == Some(target_sample_rate) {
            return;
        }

        if self.raw_presets.is_empty() {
            self.raw_presets = BundledIr::load_all()
                .expect("bundled IR bank should decode successfully during plugin setup");
        }

        self.prepared_presets.clear();
        self.max_prepared_len = 0;

        for preset in &self.raw_presets {
            let ir = preset.ir.resampled(target_sample_rate);
            self.max_prepared_len = self.max_prepared_len.max(ir.len());
            self.prepared_presets
                .push(PreparedPreset { id: preset.id, ir });
        }

        self.prepared_sample_rate = Some(target_sample_rate);
    }

    pub fn prepared_preset(&self, id: PresetId) -> &PreparedPreset {
        self.prepared_presets
            .iter()
            .find(|preset| preset.id == id)
            .expect("requested preset should have been prepared for the current host sample rate")
    }

    pub fn max_prepared_len(&self) -> usize {
        self.max_prepared_len.max(1)
    }
}
