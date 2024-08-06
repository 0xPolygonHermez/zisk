use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{fmt::Debug, sync::mpsc, thread};

use proofman::WCManager;
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_arith_32::Arith32SM;
use sm_arith_3264::Arith3264SM;
use sm_arith_64::Arith64SM;
use sm_common::{
    Arith3264Op, Arith32Op, Arith64Op, OpResult, Provable, Sessionable, Sessions, WorkerHandler,
    WorkerTask,
};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct ArithSM {
    inputs32: Mutex<Vec<Arith32Op>>,
    inputs64: Mutex<Vec<Arith64Op>>,
    arith32_sm: Arc<Arith32SM>,
    arith64_sm: Arc<Arith64SM>,
    arith3264_sm: Arc<Arith3264SM>,
}

impl ArithSM {
    pub fn new<F>(
        wcm: &mut WCManager<F>,
        arith32_sm: Arc<Arith32SM>,
        arith64_sm: Arc<Arith64SM>,
        arith3264_sm: Arc<Arith3264SM>,
    ) -> Arc<Self> {
        let arith_sm = Self {
            inputs32: Mutex::new(Vec::new()),
            inputs64: Mutex::new(Vec::new()),
            arith32_sm,
            arith64_sm,
            arith3264_sm,
        };
        let arith_sm = Arc::new(arith_sm);

        wcm.register_component(arith_sm.clone() as Arc<dyn WCComponent<F>>, None);

        arith_sm
    }
}

impl<F> WCComponent<F> for ArithSM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Arith3264Op, OpResult> for ArithSM {
    fn calculate(&self, operation: Arith3264Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Arith3264Op::Add32(a, b) => self.arith32_sm.add(a, b),
            Arith3264Op::Add64(a, b) => self.arith64_sm.add(a, b),
            Arith3264Op::Sub32(a, b) => self.arith32_sm.sub(a, b),
            Arith3264Op::Sub64(a, b) => self.arith64_sm.sub(a, b),
        }
    }

    fn prove(&self, operations: &[Arith3264Op], is_last: bool, scope: &Scope) {
        let mut inputs32 = self.inputs32.lock().unwrap();
        let mut inputs64 = self.inputs64.lock().unwrap();

        // TODO Split the operations into 32 and 64 bit operations in parallel
        for operation in operations {
            match operation {
                Arith3264Op::Add32(a, b) | Arith3264Op::Sub32(a, b) => {
                    inputs32.push(operation.clone().into());
                }
                Arith3264Op::Add64(a, b) | Arith3264Op::Sub64(a, b) => {
                    inputs64.push(operation.clone().into());
                }
            }
        }

        // The following is a way to release the lock on the inputs32 and inputs64 Mutexes asap
        // NOTE: The `inputs32` lock is released when it goes out of scope because it is shadowed
        let inputs32 = if is_last || inputs32.len() >= PROVE_CHUNK_SIZE {
            let _inputs32 = std::mem::take(&mut *inputs32);
            if _inputs32.is_empty() {
                None
            } else {
                Some(_inputs32)
            }
        } else {
            None
        };

        // NOTE: The `inputs64` lock is released when it goes out of scope because it is shadowed
        let inputs64 = if is_last || inputs64.len() >= PROVE_CHUNK_SIZE {
            let _inputs64 = std::mem::take(&mut *inputs64);
            if _inputs64.is_empty() {
                None
            } else {
                Some(_inputs64)
            }
        } else {
            None
        };

        if inputs32.is_some() {
            let arith32_s = self.arith32_sm.clone();
            scope.spawn(move |scope| {
                arith32_s.prove(&inputs32.unwrap(), is_last, scope);
            });
        }

        if inputs64.is_some() {
            let arith64_sm = self.arith64_sm.clone();
            scope.spawn(move |scope| {
                arith64_sm.prove(&inputs64.unwrap(), is_last, scope);
            });
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
