use std::{
    mem,
    sync::{Arc, Mutex},
};

use proofman::WitnessManager;
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx};
use rayon::Scope;
use witness_helpers::WitnessComponent;

use crate::{Provable, StdOpResult, StdRangeCheckOp};

const PROVE_CHUNK_SIZE: usize = 1 << 10;

pub struct StdRangeCheck {
    inputs: Mutex<Vec<StdRangeCheckOp>>,
}

impl StdRangeCheck {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let rc_sm = Self {
            inputs: Mutex::new(Vec::new()),
        };
        let rc_sm = Arc::new(rc_sm);

        wcm.register_component(rc_sm.clone() as Arc<dyn WitnessComponent<F>>, Some(air_ids));

        rc_sm
    }
}

impl<F> WitnessComponent<F> for StdRangeCheck {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
        // TODO!!!!
    }
}

impl Provable<StdRangeCheckOp, StdOpResult> for StdRangeCheck {
    fn calculate(
        &self,
        operation: StdRangeCheckOp,
    ) -> Result<StdOpResult, Box<dyn std::error::Error>> {
    }

    fn prove(&self, operations: &[StdRangeCheckOp], is_last: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);
            if is_last || inputs.len() >= PROVE_CHUNK_SIZE {
                let _inputs = mem::take(&mut *inputs);

                scope.spawn(move |scope| {
                    // TODO Calculate RC !!!!
                    println!("RCSM: Finishing the Range Check thread");
                });
            }
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
