use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{sync::mpsc, thread};

use proofman::WitnessManager;
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_common::{Arith32Op, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};
use witness_helpers::WitnessComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Arith32SM {
    inputs: Mutex<Vec<Arith32Op>>,
}

impl Arith32SM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let arith32_sm = Self { inputs: Mutex::new(Vec::new()) };
        let arith32_sm = Arc::new(arith32_sm);

        wcm.register_component(arith32_sm.clone() as Arc<dyn WitnessComponent<F>>, Some(air_ids));

        arith32_sm
    }

    pub fn add(&self, a: u32, b: u32) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok(((a + b) as u64, true))
    }

    pub fn sub(&self, a: u32, b: u32) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok(((a - b) as u64, true))
    }
}

impl<F> WitnessComponent<F> for Arith32SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Arith32Op, OpResult> for Arith32SM {
    fn calculate(&self, operation: Arith32Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Arith32Op::Add(a, b) => self.add(a, b),
            Arith32Op::Sub(a, b) => self.sub(a, b),
        }
    }

    fn prove(&self, operations: &[Arith32Op], is_last: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);
            if is_last || inputs.len() >= PROVE_CHUNK_SIZE {
                let _inputs = mem::take(&mut *inputs);

                scope.spawn(move |scope| {
                    println!(
                        "Arith32: Proving [{:?}..{:?}]",
                        _inputs[0],
                        _inputs[_inputs.len() - 1]
                    );
                    println!("Arith32: Finishing the worker thread");
                });
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: Arith32Op,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
