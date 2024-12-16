use std::sync::{Arc, Mutex};

use crate::ArithRangeTableInputs;
use zisk_pil::ArithRangeTableTrace;

pub struct ArithRangeTableSM {
    multiplicity: Mutex<Vec<u64>>,
}

impl ArithRangeTableSM {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            multiplicity: Mutex::new(vec![0; ArithRangeTableTrace::<usize>::NUM_ROWS]),
        })
    }

    pub fn process_slice(&self, inputs: &ArithRangeTableInputs) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (row, value) in inputs {
            multiplicity[row] += value;
        }
    }

    pub fn detach_multiplicity(&self) -> Vec<u64> {
        let mut multiplicity = self.multiplicity.lock().unwrap();
        std::mem::take(&mut *multiplicity)
    }
}
