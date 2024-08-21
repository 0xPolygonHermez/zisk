use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{sync::mpsc, thread};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_common::{Binary64Op, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Binary64SM {
    inputs: Mutex<Vec<Binary64Op>>,
}

impl Binary64SM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let binary64_sm = Self { inputs: Mutex::new(Vec::new()) };
        let binary64_sm = Arc::new(binary64_sm);

        wcm.register_component(binary64_sm.clone() as Arc<dyn WitnessComponent<F>>, Some(air_ids));

        binary64_sm
    }

    pub fn and(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a + b, true))
    }

    pub fn or(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a - b, true))
    }
}

impl<F> WitnessComponent<F> for Binary64SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: usize,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Binary64Op, OpResult> for Binary64SM {
    fn calculate(&self, operation: Binary64Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Binary64Op::And(a, b) => self.and(a, b),
            Binary64Op::Or(a, b) => self.or(a, b),
        }
    }

    fn prove(&self, operations: &[Binary64Op], is_last: bool, scope: &Scope) {
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
        operation: Binary64Op,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
