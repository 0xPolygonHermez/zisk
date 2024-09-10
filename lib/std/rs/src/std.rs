use std::sync::Arc;

use num_bigint::BigInt;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use crate::{Decider, RCAirData, StdProd, StdRangeCheck, StdSum};

const _PROVE_CHUNK_SIZE: usize = 1 << 12;

// struct RangeCheckInput {
//     val: BigInt,
//     min: BigInt,
//     max: BigInt,
// }

pub struct Std<F: PrimeField> {
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
            prod,
            sum,
            range_check,
        });

        // Register the STD as a component. Notice that the STD has no air associated with it
        wcm.register_component(std.clone(), None, None);

        std
    }

    /// Processes the inputs for the range check.
    pub fn range_check(&self, val: F, min: BigInt, max: BigInt) {
        // let mut inputs_range_check = self.inputs_range_check.lock().unwrap();

        // inputs_range_check.push(RangeCheckInput { val, min, max });

        // // If the maximum number of accumulated inputs is reached, the std_range_check processes them
        // if inputs_range_check.len() >= Self::MAX_ACCUMULATED {
        //     self.prove(self.inputs_range_check);
        //     inputs_range_check.clear();
        // }

        // TODO: Process the remaining inputs

        self.range_check.assign_values(val, min, max);
    }

    // /// This function should prove a batch of inputs.
    // /// When the maximum number of accumulated inputs is reached, the STD processes
    // /// the inputs in batches.
    // pub fn prove(&self, inputs) {
    //     todo!();
    //     // self.range_check.prove();
    // }
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

        // if let Err(e) = self.range_check.calculate_witness(stage, pctx, ectx) {
        //     log::error!("Range Check: Failed to calculate witness: {:?}", e);
        //     panic!();
        // }
    }
}
