use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rayon::Scope;

use proofman::WitnessManager;
use proofman_common::ProofCtx;

use crate::{RCAirData, StdProd, StdRangeCheck, StdSum};

pub struct Std<F: PrimeField> {
    range_check: Arc<StdRangeCheck<F>>,
    range_check_predecessors: AtomicU32,
}

impl<F: PrimeField> Std<F> {
    const _MY_NAME: &'static str = "STD";

    pub fn new(wcm: Arc<WitnessManager<F>>, rc_air_data: Option<Vec<RCAirData>>) -> Arc<Self> {
        // Instantiate the STD components
        StdProd::new(wcm.clone());
        StdSum::new(wcm.clone());

        // In particular, the range check component needs to be instantiated with the ids
        // of its (possibly) associated AIRs: U8Air ...
        let range_check = StdRangeCheck::new(wcm.clone(), rc_air_data);

        let std = Arc::new(Self {
            range_check,
            range_check_predecessors: AtomicU32::new(0),
        });

        std
    }

    pub fn register_predecessor(&self) {
        self.range_check_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, pctx: Arc<ProofCtx<F>>, scope: Option<&Scope>) {
        if self.range_check_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.range_check.drain_inputs(pctx, scope);
        }
    }

    /// Processes the inputs for the range check.
    pub fn range_check(&self, val: F, min: BigInt, max: BigInt) {
        self.range_check.assign_values(val, min, max);
    }
}
