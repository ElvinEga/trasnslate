mod bundled;

pub use bundled::{BundledIr, BundledIrId, StereoIr};

use std::sync::Arc;

#[derive(Debug)]
pub struct IrState {
    _active_id: BundledIrId,
    _active_path: &'static str,
    active: Arc<StereoIr>,
}

impl IrState {
    pub fn active(&self) -> &StereoIr {
        self.active.as_ref()
    }
}

impl Default for IrState {
    fn default() -> Self {
        let bundled = BundledIr::load(BundledIrId::PlaceholderRoom)
            .expect("bundled placeholder IR should decode successfully");

        Self {
            _active_id: bundled.id,
            _active_path: bundled.path,
            active: Arc::new(bundled.ir),
        }
    }
}
