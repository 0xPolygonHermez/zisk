use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use zisk_common::create_atomic_vec;
use zisk_pil::MemAlignRomTrace;

pub struct MemAlignRomSM {
    multiplicity: Vec<AtomicU64>, // row_num -> multiplicity
    calculated: AtomicBool,
}

impl MemAlignRomSM {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            multiplicity: create_atomic_vec(MemAlignRomTrace::<usize>::NUM_ROWS),
            calculated: AtomicBool::new(false),
        })
    }

    pub fn detach_multiplicity(&self) -> &[AtomicU64] {
        &self.multiplicity
    }

    pub fn set_calculated(&self) {
        self.calculated.store(true, Ordering::Relaxed);
    }

    pub fn reset_calculated(&self) {
        self.calculated.store(false, Ordering::Relaxed);
    }

    pub fn update_padding_row(&self, padding_len: u64) {
        // Update entry at the padding row (pos = 0) with the given padding length
        self.update_multiplicity_by_row_idx(0, padding_len);
    }

    pub fn update_multiplicity_by_row_idx(&self, row_idx: u64, mul: u64) {
        debug_assert!(row_idx < MemAlignRomTrace::<usize>::NUM_ROWS as u64);
        if self.calculated.load(Ordering::Relaxed) {
            return;
        }
        self.multiplicity[row_idx as usize].fetch_add(mul, Ordering::Relaxed);
    }
}
