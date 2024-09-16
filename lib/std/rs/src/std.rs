use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rayon::Scope;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use crate::{Decider, RCAirData, StdProd, StdRangeCheck, StdSum};

pub struct Std<F: PrimeField> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // STD components
    prod: Arc<StdProd<F>>,
    sum: Arc<StdSum<F>>,
    range_check: Arc<StdRangeCheck<F>>,
}

impl<F: PrimeField> Std<F> {
    const _MY_NAME: &'static str = "STD";

    pub fn new(wcm: &mut WitnessManager<F>, rc_air_data: Option<Vec<RCAirData>>) -> Arc<Self> {
        // Instantiate the STD components
        let prod = StdProd::new();
        let sum = StdSum::new();

        // In particular, the range check component needs to be instantiated with the ids
        // of its (possibly) associated AIRs: U8Air ...
        let range_check = StdRangeCheck::new(wcm, rc_air_data);

        let std = Arc::new(Self {
            registered_predecessors: AtomicU32::new(0),
            prod,
            sum,
            range_check,
        });

        // Register the STD as a component. Notice that the STD has no air associated with it
        wcm.register_component(std.clone(), None, None);

        std
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, pctx: &mut ProofCtx<F>, scope: Option<&Scope>) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.range_check.drain_inputs(pctx, scope);
        }
    }

    /// Processes the inputs for the range check.
    pub fn range_check(&self, val: F, min: BigInt, max: BigInt) {
        self.range_check.assign_values(val, min, max);
    }
}

impl<F: PrimeField> WitnessComponent<F> for Std<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, _ectx: &ExecutionCtx, sctx: &SetupCtx) {
        // Run the deciders of the components on the correct stage to see if they need to calculate their witness
        self.prod.decide(sctx, pctx);
        self.sum.decide(sctx, pctx);
        self.range_check.decide(sctx, pctx);
    }

    fn calculate_witness(
        &self,
        stage: u32,
        _air_instance: Option<usize>,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        if let Err(e) = self.prod.calculate_witness(stage, pctx, sctx) {
            log::error!("Prod: Failed to calculate witness: {:?}", e);
            panic!();
        }

        if let Err(e) = self.sum.calculate_witness(stage, pctx, sctx) {
            log::error!("Sum: Failed to calculate witness: {:?}", e);
            panic!();
        }
    }
}
