use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

use std::{fmt::Debug, sync::mpsc, thread};

use proofman::WCManager;
use sm_binary_32::Binary32SM;
use sm_binary_3264::Binary3264SM;
use sm_binary_64::Binary64SM;
use sm_common::{
    Binary3264Op, Binary32Op, Binary64Op, Provable, Sessionable, Sessions, WorkerHandler,
    WorkerTask, ZiskResult,
};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct BinarySM {
    inputs32: Arc<RwLock<Vec<Binary32Op>>>,
    inputs64: Arc<RwLock<Vec<Binary64Op>>>,
    last_proved_idx32: AtomicUsize,
    last_proved_idx64: AtomicUsize,
    binary32_sm: Arc<Binary32SM>,
    binary64_sm: Arc<Binary64SM>,
    binary3264_sm: Arc<Binary3264SM>,
    sessions: Arc<Sessions>,
    opened_sessions: Vec<usize>,
}

impl BinarySM {
    pub fn new<F>(
        wcm: &mut WCManager<F>,
        sessions: Arc<Sessions>,
        binary32_sm: Arc<Binary32SM>,
        binary64_sm: Arc<Binary64SM>,
        binary3264_sm: Arc<Binary3264SM>,
    ) -> Arc<Self> {
        let inputs32 = Arc::new(RwLock::new(Vec::new()));
        let inputs64 = Arc::new(RwLock::new(Vec::new()));

        let opened_sessions = vec![
            sessions.open_session(binary32_sm.clone()),
            sessions.open_session(binary64_sm.clone()),
            sessions.open_session(binary3264_sm.clone()),
        ];

        let binary_sm = Self {
            inputs32,
            inputs64,
            last_proved_idx32: AtomicUsize::new(0),
            last_proved_idx64: AtomicUsize::new(0),
            binary32_sm,
            binary64_sm,
            binary3264_sm,
            sessions,
            opened_sessions,
        };
        let binary_sm = Arc::new(binary_sm);

        wcm.register_component(binary_sm.clone() as Arc<dyn WCComponent<F>>, None);

        binary_sm
    }
}

impl<F> WCComponent<F> for BinarySM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &common::AirInstance,
        pctx: &mut common::ProofCtx<F>,
        ectx: &common::ExecutionCtx,
    ) {
    }
}

impl Provable<Binary3264Op, ZiskResult> for BinarySM {
    fn calculate(&self, operation: Binary3264Op) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        match operation {
            Binary3264Op::And32(a, b) => self.binary32_sm.and(a, b),
            Binary3264Op::And64(a, b) => self.binary64_sm.and(a, b),
        }
    }

    fn prove(&self, operations: &[Binary3264Op]) {
        // Create a scoped block to hold the write lock only the necessary
        let (num_inputs32, num_inputs64) = {
            let mut inputs32 = self.inputs32.write().unwrap();
            let mut inputs64 = self.inputs64.write().unwrap();
            for operation in operations {
                match operation {
                    Binary3264Op::And32(a, b) => {
                        inputs32.push(operation.clone().into());
                    }
                    Binary3264Op::And64(a, b) => {
                        inputs64.push(operation.clone().into());
                    }
                }
            }
            (inputs32.len(), inputs64.len())
        };

        if num_inputs32 % PROVE_CHUNK_SIZE == 0 {
            let last_proved_idx = self.last_proved_idx32.load(Ordering::Relaxed);
            self.binary32_sm.prove(&self.inputs32.read().unwrap()[last_proved_idx..num_inputs32]);
            self.last_proved_idx32.store(num_inputs32, Ordering::Relaxed);
        }

        if num_inputs64 % PROVE_CHUNK_SIZE == 0 {
            let last_proved_idx = self.last_proved_idx64.load(Ordering::Relaxed);
            self.binary64_sm.prove(&self.inputs64.read().unwrap()[last_proved_idx..num_inputs64]);
            self.last_proved_idx64.store(num_inputs64, Ordering::Relaxed);
        }
    }

    fn calculate_prove(
        &self,
        operation: Binary3264Op,
    ) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for BinarySM {
    fn when_closed(&self) {
        // Prove remaining inputs if any
        // TODO We need to prove the remaining inputs. If the number of inputs32 and inputs64 fits
        // in a single proof, we can prove them together using 3264.
        let num_inputs32 = { self.inputs32.read().unwrap().len() };
        let last_proved_idx32 = self.last_proved_idx32.load(Ordering::Relaxed);
        self.binary32_sm.prove(&self.inputs32.read().unwrap()[last_proved_idx32..num_inputs32]);

        let num_inputs64 = { self.inputs64.read().unwrap().len() };
        let last_proved_idx64 = self.last_proved_idx64.load(Ordering::Relaxed);
        self.binary64_sm.prove(&self.inputs64.read().unwrap()[last_proved_idx64..num_inputs64]);

        // Close open sessions for the current thread
        for session_id in &self.opened_sessions {
            self.sessions.close_session(*session_id).expect("Failed to close session");
        }
    }
}
