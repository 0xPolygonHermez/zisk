use std::sync::{Arc, Mutex};

use crate::ArithTableInputs;
use zisk_pil::ArithTableTrace;

use p3_field::PrimeField;

pub struct ArithTableSM {
    pub multiplicity: Mutex<Vec<u64>>,
}

impl ArithTableSM {
    pub fn new<F: PrimeField>() -> Arc<Self> {
        Arc::new(Self { multiplicity: Mutex::new(vec![0; ArithTableTrace::<F>::NUM_ROWS]) })
    }

    pub fn process_slice(&self, inputs: &ArithTableInputs) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (row, value) in inputs {
            multiplicity[row] += value;
        }
    }
}
