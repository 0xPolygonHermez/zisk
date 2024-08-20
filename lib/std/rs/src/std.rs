use std::sync::{Arc, Mutex};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, AirInstanceCtx, ExecutionCtx, ProofCtx};
use rayon::Scope;

use crate::{Provable, StdOp, StdOpResult, StdProd, StdRangeCheck, StdRangeCheckOp, StdSum};

const PROVE_CHUNK_SIZE: usize = 1 << 10;

pub struct Std {
    inputs_rc: Mutex<Vec<(u64, u64, u64)>>,

    // div_lib_sm: Arc<StdDivLib>,
    prod: Arc<StdProd>,
    sum: Arc<StdSum>,
    // range_check: Arc<StdRangeCheck>,
}

impl Std {
    pub fn new<F>(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        // let div_lib_sm = StdDivLib::new(&mut wcm, None);
        let prod = Arc::new(StdProd);
        let sum = Arc::new(StdSum);

        let range_check = StdRangeCheck::new(&mut wcm, [1000]); // TODO!!!!! Change id

        let std_sm = Self {
            inputs_rc: Mutex::new(Vec::new()),
            // div_lib_sm,
            prod,
            sum,
            // range_check,
        };
        let std_sm = Arc::new(std_sm);

        wcm.register_component(std_sm.clone() as Arc<dyn WitnessComponent<F>>, None);

        std_sm
    }
}

impl<F> WitnessComponent<F> for Std {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstanceCtx<F>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {
        self.prod.decide(pctx.pilout, air_instance, pctx, ectx);
        self.sum.decide(pctx.pilout, air_instance, pctx, ectx);
    }
}

impl Provable<StdRangeCheckOp, StdOpResult> for Std {
    fn calculate(
        &self,
        operation: StdRangeCheckOp,
    ) -> Result<StdOpResult, Box<dyn std::error::Error>> {
    }

    fn prove(&self, operations: &[StdRangeCheckOp], is_last: bool, scope: &Scope) {
        let mut inputs = self.inputs_rc.lock().unwrap();

        for operation in operations {
            match operation {
                StdOp::RangeCheck(value, min, max) => {
                    inputs.push((value, min, max));
                }
            }
        }

        if is_last || inputs.len() >= PROVE_CHUNK_SIZE {
            let _inputs = std::mem::take(&mut *inputs);

            let rc_sm = self.rc_sm.clone();
            scope.spawn(move |scope| {
                rc_sm.prove(&inputs.unwrap(), is_last, scope);
            });
        }
    }

    fn calculate_prove(
        &self,
        operation: StdRangeCheckOp,
        is_last: bool,
        scope: &Scope,
    ) -> Result<StdOpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
