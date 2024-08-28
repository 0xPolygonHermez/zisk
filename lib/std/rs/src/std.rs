use std::{fmt::Debug, hash::Hash, sync::Arc};

use num_bigint::BigInt;
use p3_field::{Field, PrimeField};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;

use crate::{Decider, StdProd, StdRangeCheck, StdSum};

pub struct Std<F> {
    prod: Arc<StdProd<F>>,
    sum: Arc<StdSum<F>>,
    range_check: Arc<StdRangeCheck<F>>,
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy + Clone + PartialOrd + PartialEq + Eq + Hash + Field + 'static> Std<F> {
    const _MY_NAME: &'static str = "STD";

    // TODO
    // const CALLBACK_SIZE: usize = 2usize.pow(16);
    // const MAX_ACCUMULATED: usize = 2usize.pow(21);

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let prod = Arc::new(StdProd::new());
        let sum = Arc::new(StdSum::new());
        let range_check = Arc::new(StdRangeCheck::<F>::new());

        let std = Arc::new(Self {
            prod,
            sum,
            range_check,
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(std.clone() as Arc<dyn WitnessComponent<F>>, None);

        std
    }

    pub fn execute(&self, _pctx: &mut ProofCtx<F>, _ectx: &mut ExecutionCtx) {
        todo!();
    }

    pub fn setup_range_check(
        &self,
        air_instance_idx: usize,
        pctx: &mut ProofCtx<F>,
        sctx: &SetupCtx,
    ) {
        let air_instances = pctx.air_instances.read().unwrap();
        let air_instance = &air_instances[air_instance_idx];

        self.range_check
            .register_ranges(air_instance.air_group_id, air_instance.air_id, sctx);
    }

    // TODO: Could we set min and max to be signed integers [-p,p] instead of F?
    /// Processes the inputs for the range check.
    pub fn range_check(&self, val: BigInt, min: BigInt, max: BigInt) {
        self.range_check.assign_values(val, min, max);
    }

    /// This function should prove a batch of inputs.
    /// When the maximum number of accumulated inputs is reached, the STD processes
    /// the inputs in batches.
    pub fn prove(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx) {
        todo!();
        // self.range_check.prove();
    }
}

impl<F: PrimeField> WitnessComponent<F> for Std<F> {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: usize,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        // Run the deciders of the components on the correct stage to see if they need to calculate their witness
        self.prod.decide(stage, air_instance, pctx, ectx, sctx);
        self.sum.decide(stage, air_instance, pctx, ectx, sctx);
        self.range_check.decide(stage, air_instance, pctx, ectx, sctx);
    }
}
