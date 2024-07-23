use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

use std::{fmt::Debug, sync::mpsc, thread};

use proofman::WCManager;
use sm_arith_32::Arith32SM;
use sm_arith_3264::Arith3264SM;
use sm_arith_64::Arith64SM;
use sm_common::{
    Arith3264Op, Arith32Op, Arith64Op, Provable, Sessionable, Sessions, WorkerHandler, WorkerTask,
    ZiskResult,
};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct ArithSM {
    inputs32: Arc<RwLock<Vec<Arith32Op>>>,
    inputs64: Arc<RwLock<Vec<Arith64Op>>>,
    last_proved_idx32: AtomicUsize,
    last_proved_idx64: AtomicUsize,
    arith32_sm: Arc<Arith32SM>,
    arith64_sm: Arc<Arith64SM>,
    arith3264_sm: Arc<Arith3264SM>,
    sessions: Arc<Sessions>,
    opened_sessions: Vec<usize>,
}

impl ArithSM {
    pub fn new<F>(
        wcm: &mut WCManager<F>,
        sessions: Arc<Sessions>,
        arith32_sm: Arc<Arith32SM>,
        arith64_sm: Arc<Arith64SM>,
        arith3264_sm: Arc<Arith3264SM>,
    ) -> Arc<Self> {
        let inputs32 = Arc::new(RwLock::new(Vec::new()));
        let inputs64 = Arc::new(RwLock::new(Vec::new()));

        let opened_sessions = vec![
            sessions.open_session(arith32_sm.clone()),
            sessions.open_session(arith64_sm.clone()),
            sessions.open_session(arith3264_sm.clone()),
        ];

        let arith_sm = Self {
            inputs32,
            inputs64,
            last_proved_idx32: AtomicUsize::new(0),
            last_proved_idx64: AtomicUsize::new(0),
            arith32_sm,
            arith64_sm,
            arith3264_sm,
            sessions,
            opened_sessions,
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
        air_instance: &common::AirInstance,
        pctx: &mut common::ProofCtx<F>,
        ectx: &common::ExecutionCtx,
    ) {
    }
}

impl Provable<Arith3264Op, ZiskResult> for ArithSM {
    fn calculate(&self, operation: Arith3264Op) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        match operation {
            Arith3264Op::Add32(a, b) => self.arith32_sm.add(a, b),
            Arith3264Op::Add64(a, b) => self.arith64_sm.add(a, b),
        }
    }

    fn prove(&self, operations: &[Arith3264Op]) {
        // Create a scoped block to hold the write lock only the necessary
        let (num_inputs32, num_inputs64) = {
            let mut inputs32 = self.inputs32.write().unwrap();
            let mut inputs64 = self.inputs64.write().unwrap();
            for operation in operations {
                match operation {
                    Arith3264Op::Add32(a, b) => {
                        inputs32.push(operation.clone().into());
                    }
                    Arith3264Op::Add64(a, b) => {
                        inputs64.push(operation.clone().into());
                    }
                }
            }
            (inputs32.len(), inputs64.len())
        };

        if num_inputs32 % PROVE_CHUNK_SIZE == 0 {
            let last_proved_idx = self.last_proved_idx32.load(Ordering::Relaxed);
            self.arith32_sm.prove(&self.inputs32.read().unwrap()[last_proved_idx..num_inputs32]);
            self.last_proved_idx32.store(num_inputs32, Ordering::Relaxed);
        }

        if num_inputs64 % PROVE_CHUNK_SIZE == 0 {
            let last_proved_idx = self.last_proved_idx64.load(Ordering::Relaxed);
            self.arith64_sm.prove(&self.inputs64.read().unwrap()[last_proved_idx..num_inputs64]);
            self.last_proved_idx64.store(num_inputs64, Ordering::Relaxed);
        }
    }

    fn calculate_prove(
        &self,
        operation: Arith3264Op,
    ) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for ArithSM {
    fn when_closed(&self) {
        // Prove remaining inputs if any
        // TODO We need to prove the remaining inputs. If the number of inputs32 and inputs64 fits
        // in a single proof, we can prove them together using 3264.
        let num_inputs32 = { self.inputs32.read().unwrap().len() };
        let last_proved_idx32 = self.last_proved_idx32.load(Ordering::Relaxed);
        self.arith32_sm.prove(&self.inputs32.read().unwrap()[last_proved_idx32..num_inputs32]);

        let num_inputs64 = { self.inputs64.read().unwrap().len() };
        let last_proved_idx64 = self.last_proved_idx64.load(Ordering::Relaxed);
        self.arith64_sm.prove(&self.inputs64.read().unwrap()[last_proved_idx64..num_inputs64]);

        // Close open sessions for the current thread
        for session_id in &self.opened_sessions {
            self.sessions.close_session(*session_id).expect("Failed to close session");
        }
    }
}
