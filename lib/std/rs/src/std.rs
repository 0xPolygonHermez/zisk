use std::{hash::Hash,sync::Arc,fmt::Debug};

use p3_field::{AbstractField, Field};
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;

use crate::{Decider, StdProd, StdSum/*, StdRangeCheck*/};

pub struct Std<F> {
    prod: Arc<StdProd<F>>,
    sum: Arc<StdSum<F>>,
    // range_check: Arc<StdRangeCheck<F>>,
    // TODO! REMOVE this line when range_check is uncommented
    _phantom: std::marker::PhantomData<F>,
}

impl<F: AbstractField + Copy + Clone + PartialEq + Eq + Hash + Field + 'static> Std<F> {
    const MY_NAME: &'static str = "STD";

    // TODO
    // const CALLBACK_SIZE: usize = 2usize.pow(16);
    // const MAX_ACCUMULATED: usize = 2usize.pow(21);

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let prod = Arc::new(StdProd::new());
        let sum = Arc::new(StdSum::new());
        // let range_check = Arc::new(StdRangeCheck::<F>::new());

        let std = Arc::new(Self {
            prod,
            sum,
            // range_check,
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(std.clone() as Arc<dyn WitnessComponent<F>>, None);

        std
    }

    pub fn execute(
        &self,
        pctx: &mut ProofCtx<F>,
        ectx: &mut ExecutionCtx,
    ) {
        todo!();
    }

    /// This function should prove a batch of inputs.
    /// When the maximum number of accumulated inputs is reached, the STD processes
    /// the inputs in batches.
    fn prove(
        &self,
        pctx: &ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {

    }

    // pub fn setup_range_check(&self, air_instance: &AirInstanceCtx<F>, pctx: &ProofCtx<F>) {
    //     self.range_check.setup(air_instance.air_group_id.try_into().expect("TBD"), air_instance.air_id.try_into().expect("TBD"), pctx.pilout);
    // }

    // TODO: Could we set min and max to be signed integers [-p,p] instead of F?
    // pub fn range_check(&self, val: F, min: F, max: F) {
    //     self.range_check.assign_values(val, min, max);
    // }
}

impl<F: Copy + Debug + Field> WitnessComponent<F> for Std<F> {
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
        // self.range_check.decide(pctx.pilout, air_instance, pctx, ectx, sctx);
    }
}
