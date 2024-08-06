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
use sm_common::{Binary3264Op, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Binary3264SM {
    inputs: Mutex<Vec<Binary3264Op>>,
}

impl Binary3264SM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let binary3264_sm = Self { inputs: Mutex::new(Vec::new()) };
        let binary3264_sm = Arc::new(binary3264_sm);

        wcm.register_component(binary3264_sm.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        binary3264_sm
    }

    pub fn and32(&self, a: u32, b: u32) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok(((a + b) as u64, true))
    }

    pub fn and64(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a + b, true))
    }

    pub fn or32(&self, a: u32, b: u32) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok(((a - b) as u64, true))
    }

    pub fn or64(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a - b, true))
    }
}

impl<F> WCComponent<F> for Binary3264SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Binary3264Op, OpResult> for Binary3264SM {
    fn calculate(&self, operation: Binary3264Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Binary3264Op::And32(a, b) => self.and32(a, b),
            Binary3264Op::And64(a, b) => self.and64(a, b),
            Binary3264Op::Or32(a, b) => self.or32(a, b),
            Binary3264Op::Or64(a, b) => self.or64(a, b),
        }
    }

    fn prove(&self, operations: &[Binary3264Op], is_last: bool, scope: &Scope) {
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
        operation: Binary3264Op,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
