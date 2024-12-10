use std::sync::{Arc, Mutex};

use crate::ArithRangeTableInputs;
use zisk_pil::ArithRangeTableTrace;

use p3_field::PrimeField;

pub struct ArithRangeTableSM {
    pub multiplicity: Mutex<Vec<u64>>,
}

impl ArithRangeTableSM {
    pub fn new<F: PrimeField>() -> Arc<Self> {
        Arc::new(Self { multiplicity: Mutex::new(vec![0; ArithRangeTableTrace::<F>::NUM_ROWS]) })
    }

    pub fn process_slice(&self, inputs: &ArithRangeTableInputs) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (row, value) in inputs {
            multiplicity[row] += value;
        }
    }
}
