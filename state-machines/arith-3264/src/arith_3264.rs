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
use sm_common::{Arith3264Op, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Arith3264SM {
    inputs: Mutex<Vec<Arith3264Op>>,
}

impl Arith3264SM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let arith3264_sm = Self { inputs: Mutex::new(Vec::new()) };
        let arith3264_sm = Arc::new(arith3264_sm);

        wcm.register_component(arith3264_sm.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        arith3264_sm
    }

    pub fn add32(&self, a: u32, b: u32) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok(((a + b) as u64, true))
    }

    pub fn add64(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a + b, true))
    }

    pub fn sub32(&self, a: u32, b: u32) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok(((a - b) as u64, true))
    }

    pub fn sub64(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a - b, true))
    }
}

impl<F> WCComponent<F> for Arith3264SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Arith3264Op, OpResult> for Arith3264SM {
    fn calculate(&self, operation: Arith3264Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Arith3264Op::Add32(a, b) => self.add32(a, b),
            Arith3264Op::Add64(a, b) => self.add64(a, b),
            Arith3264Op::Sub32(a, b) => self.sub32(a, b),
            Arith3264Op::Sub64(a, b) => self.sub64(a, b),
        }
    }

    fn prove(&self, operations: &[Arith3264Op], is_last: bool, scope: &Scope) {
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
        operation: Arith3264Op,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}