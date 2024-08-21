use std::{
    mem,
    sync::{Arc, Mutex},
};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_common::{Binary32Op, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Binary32SM {
    inputs: Mutex<Vec<Binary32Op>>,
}

impl Binary32SM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let binary32_sm = Self { inputs: Mutex::new(Vec::new()) };
        let binary32_sm = Arc::new(binary32_sm);

        wcm.register_component(binary32_sm.clone() as Arc<dyn WitnessComponent<F>>, Some(air_ids));

        binary32_sm
    }

    pub fn and(&self, a: u32, b: u32) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok(((a & b) as u64, true))
    }

    pub fn or(&self, a: u32, b: u32) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok(((a | b) as u64, true))
    }
}

impl<F> WitnessComponent<F> for Binary32SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: usize,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Binary32Op, OpResult> for Binary32SM {
    fn calculate(&self, operation: Binary32Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Binary32Op::And(a, b) => self.and(a, b),
            Binary32Op::Or(a, b) => self.or(a, b),
        }
    }

    fn prove(&self, operations: &[Binary32Op], is_last: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);
            if is_last || inputs.len() >= PROVE_CHUNK_SIZE {
                let _inputs = mem::take(&mut *inputs);

                scope.spawn(move |scope| {
                    println!(
                        "Binary32: Proving [{:?}..{:?}]",
                        _inputs[0],
                        _inputs[_inputs.len() - 1]
                    );
                    println!("Binary32: Finishing the worker thread");
                });
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: Binary32Op,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
