use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{sync::mpsc, thread};

use proofman::WCManager;
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_common::{Arith64Op, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Arith64SM {
    inputs: Mutex<Vec<Arith64Op>>,
}

impl Arith64SM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let arith64_sm = Self { inputs: Mutex::new(Vec::new()) };
        let arith64_sm = Arc::new(arith64_sm);

        wcm.register_component(arith64_sm.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        arith64_sm
    }

    pub fn add(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a + b, true))
    }

    pub fn sub(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a - b, true))
    }
}

impl<F> WCComponent<F> for Arith64SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Arith64Op, OpResult> for Arith64SM {
    fn calculate(&self, operation: Arith64Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Arith64Op::Add(a, b) => self.add(a, b),
            Arith64Op::Sub(a, b) => self.sub(a, b),
        }
    }

    fn prove(&self, operations: &[Arith64Op], is_last: bool, scope: &Scope) {
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
        operation: Arith64Op,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}