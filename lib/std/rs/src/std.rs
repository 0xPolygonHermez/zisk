use std::sync::{Arc, Mutex};
use std::{hash::Hash};

use p3_field::AbstractField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use rayon::Scope;

// use crate::{Provable, StdOp, StdOpResult, StdProd, StdRangeCheck, StdSum};
use crate::{StdProd, StdRangeCheck, StdSum};

const PROVE_CHUNK_SIZE: usize = 1 << 10;

pub struct Std<F> {
    inputs_rc: Mutex<Vec<(u64, u64, u64)>>, // Is this necessary?
    prod: Arc<StdProd>,
    sum: Arc<StdSum>,
    range_check: Arc<StdRangeCheck<F>>,
}

impl<F: AbstractField + Copy + Clone + PartialEq + Eq + Hash + 'static>
    Std<F>
{
    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let prod = Arc::new(StdProd);
        let sum = Arc::new(StdSum);
        let range_check = Arc::new(StdRangeCheck::<F>::new());

        let std = Arc::new(Self {
            inputs_rc: Mutex::new(Vec::new()),
            prod,
            sum,
            range_check,
        });

        wcm.register_component(std.clone() as Arc<dyn WitnessComponent<F>>, None);

        std
    }

    pub fn setup_range_check(&self, air_instance: &AirInstanceCtx<F>, pctx: &ProofCtx<F>) {
        self.range_check.setup(air_instance.air_group_id.try_into().expect("TBD"), air_instance.air_id.try_into().expect("TBD"), pctx.pilout);
    }

    // TODO: Could we set min and max to be signed integers [-p,p] instead of F?
    pub fn range_check(&self, val: F, min: F, max: F) {
        self.range_check.assign_values(val, min, max);
    }
}

impl<F> WitnessComponent<F> for Std<F> {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: usize,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {
        // self.prod.decide(pctx.pilout, air_instance, pctx, ectx);
        // self.sum.decide(pctx.pilout, air_instance, pctx, ectx);
    }
}

// impl Provable<StdRangeCheckOp, StdOpResult> for Std {
//     fn calculate(
//         &self,
//         operation: StdRangeCheckOp,
//     ) -> Result<StdOpResult, Box<dyn std::error::Error>> {
//     }

//     fn prove(&self, operations: &[StdRangeCheckOp], is_last: bool, scope: &Scope) {
//         let mut inputs = self.inputs_rc.lock().unwrap();

//         for operation in operations {
//             match operation {
//                 StdOp::RangeCheck(value, min, max) => {
//                     inputs.push((value, min, max));
//                 }
//             }
//         }

//         if is_last || inputs.len() >= PROVE_CHUNK_SIZE {
//             let _inputs = std::mem::take(&mut *inputs);

//             let rc_sm = self.rc_sm.clone();
//             scope.spawn(move |scope| {
//                 rc_sm.prove(&inputs.unwrap(), is_last, scope);
//             });
//         }
//     }

//     fn calculate_prove(
//         &self,
//         operation: StdRangeCheckOp,
//         is_last: bool,
//         scope: &Scope,
//     ) -> Result<StdOpResult, Box<dyn std::error::Error>> {
//         let result = self.calculate(operation.clone());
//         self.prove(&[operation], is_last, scope);
//         result
//     }
// }
