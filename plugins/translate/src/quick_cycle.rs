use crate::params::PresetId;
use nih_plug::prelude::Enum;
use std::array;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const PRESET_COUNT: usize = 9;
const NONE_INDEX: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickCycleAction {
    None,
    Previous,
    StartOrCycle,
    PauseOrStop,
    Next,
    ReturnToReference,
}

#[derive(Debug, Clone, Copy)]
pub struct CycleSlot {
    pub preset: PresetId,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct QuickCycleSnapshot {
    pub slots: [CycleSlot; PRESET_COUNT],
}

#[derive(Debug)]
pub struct QuickCycleShared {
    enabled: [AtomicBool; PRESET_COUNT],
    order: [AtomicU32; PRESET_COUNT],
    pending_action: AtomicU32,
    current_preset: AtomicU32,
    next_preset: AtomicU32,
    running: AtomicBool,
}

impl Default for QuickCycleShared {
    fn default() -> Self {
        Self {
            enabled: array::from_fn(|_| AtomicBool::new(true)),
            order: array::from_fn(|index| AtomicU32::new(index as u32)),
            pending_action: AtomicU32::new(QuickCycleAction::None.code()),
            current_preset: AtomicU32::new(NONE_INDEX),
            next_preset: AtomicU32::new(NONE_INDEX),
            running: AtomicBool::new(false),
        }
    }
}

impl QuickCycleShared {
    pub fn snapshot(&self) -> QuickCycleSnapshot {
        QuickCycleSnapshot {
            slots: array::from_fn(|slot| CycleSlot {
                preset: PresetId::from_index(
                    self.order[slot].load(Ordering::Relaxed) as usize % PRESET_COUNT,
                ),
                enabled: self.enabled[slot].load(Ordering::Relaxed),
            }),
        }
    }

    pub fn is_slot_enabled(&self, slot: usize) -> bool {
        self.enabled[slot].load(Ordering::Relaxed)
    }

    pub fn set_slot_enabled(&self, slot: usize, enabled: bool) {
        self.enabled[slot].store(enabled, Ordering::Relaxed);
    }

    pub fn move_up(&self, slot: usize) {
        if slot == 0 || slot >= PRESET_COUNT {
            return;
        }

        let current = self.order[slot].load(Ordering::Relaxed);
        let previous = self.order[slot - 1].load(Ordering::Relaxed);
        self.order[slot - 1].store(current, Ordering::Relaxed);
        self.order[slot].store(previous, Ordering::Relaxed);
    }

    pub fn move_down(&self, slot: usize) {
        if slot + 1 >= PRESET_COUNT {
            return;
        }

        let current = self.order[slot].load(Ordering::Relaxed);
        let next = self.order[slot + 1].load(Ordering::Relaxed);
        self.order[slot + 1].store(current, Ordering::Relaxed);
        self.order[slot].store(next, Ordering::Relaxed);
    }

    pub fn request_action(&self, action: QuickCycleAction) {
        self.pending_action.store(action.code(), Ordering::Relaxed);
    }

    pub fn take_action(&self) -> QuickCycleAction {
        QuickCycleAction::from_code(
            self.pending_action
                .swap(QuickCycleAction::None.code(), Ordering::Relaxed),
        )
    }

    pub fn set_status(&self, current: Option<PresetId>, next: Option<PresetId>, running: bool) {
        self.current_preset
            .store(current.map_or(NONE_INDEX, preset_to_u32), Ordering::Relaxed);
        self.next_preset
            .store(next.map_or(NONE_INDEX, preset_to_u32), Ordering::Relaxed);
        self.running.store(running, Ordering::Relaxed);
    }

    pub fn current_preset(&self) -> Option<PresetId> {
        preset_from_u32(self.current_preset.load(Ordering::Relaxed))
    }

    pub fn next_preset(&self) -> Option<PresetId> {
        preset_from_u32(self.next_preset.load(Ordering::Relaxed))
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

impl QuickCycleAction {
    fn code(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Previous => 1,
            Self::StartOrCycle => 2,
            Self::PauseOrStop => 3,
            Self::Next => 4,
            Self::ReturnToReference => 5,
        }
    }

    fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Previous,
            2 => Self::StartOrCycle,
            3 => Self::PauseOrStop,
            4 => Self::Next,
            5 => Self::ReturnToReference,
            _ => Self::None,
        }
    }
}

fn preset_to_u32(preset: PresetId) -> u32 {
    preset.to_index() as u32
}

fn preset_from_u32(value: u32) -> Option<PresetId> {
    if value == NONE_INDEX {
        None
    } else {
        Some(PresetId::from_index(value as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::PRESET_COUNT;
    use crate::params::PresetId;
    use nih_plug::prelude::Enum;

    #[test]
    fn preset_count_matches_enum_variants() {
        assert_eq!(PRESET_COUNT, PresetId::variants().len());
    }
}
