use std::hash::Hash;
use std::sync::{Arc, Mutex};

use p3_field::{AbstractField, Field};
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;

// use crate::{Provable, StdOp, StdOpResult, StdProd, StdRangeCheck, StdSum};
use crate::{StdProd /*StdRangeCheck, StdSum*/};

const PROVE_CHUNK_SIZE: usize = 1 << 10;

pub struct Std<F> {
    inputs_rc: Mutex<Vec<(u64, u64, u64)>>, // Is this necessary?
    prod: Arc<StdProd<F>>,
    // sum: Arc<StdSum>,
    // range_check: Arc<StdRangeCheck<F>>,
    // TODO! REMOVE this line when range_check is uncommented
    _phantom: std::marker::PhantomData<F>,
}

impl<F: AbstractField + Copy + Clone + PartialEq + Eq + Hash + Field + 'static> Std<F> {
    // TODO: Implement execute function

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let prod = Arc::new(StdProd::new());
        // let sum = Arc::new(StdSum);
        // let range_check = Arc::new(StdRangeCheck::<F>::new());

        let std = Arc::new(Self {
            inputs_rc: Mutex::new(Vec::new()),
            prod,
            // sum,
            // range_check,
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(std.clone() as Arc<dyn WitnessComponent<F>>, None);

        std
    }

    // pub fn setup_range_check(&self, air_instance: &AirInstanceCtx<F>, pctx: &ProofCtx<F>) {
    //     self.range_check.setup(
    //         air_instance.air_group_id.try_into().expect("TBD"),
    //         air_instance.air_id.try_into().expect("TBD"),
    //         pctx.pilout,
    //     );
    // }

    // // TODO: Could we set min and max to be signed integers [-p,p] instead of F?
    // pub fn range_check(&self, val: F, min: F, max: F) {
    //     self.range_check.assign_values(val, min, max);
    // }
}

impl<F> WitnessComponent<F> for Std<F> {
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
        // self.range_check.decide(pctx.pilout, air_instance, pctx, ectx, sctx);
        // self.sum.decide(pctx.pilout, air_instance, pctx, ectx, sctx);
    }
}
