use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{fmt::Debug, sync::mpsc, thread};

use proofman::WCManager;
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
        let opened_sessions = vec![
            sessions.open_session(arith32_sm.clone()),
            sessions.open_session(arith64_sm.clone()),
            sessions.open_session(arith3264_sm.clone()),
        ];

        let arith_sm = Self {
            inputs32: Mutex::new(Vec::new()),
            inputs64: Mutex::new(Vec::new()),
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

impl Provable<Arith3264Op, OpResult> for ArithSM {
    fn calculate(&self, operation: Arith3264Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Arith3264Op::Add32(a, b) => self.arith32_sm.add(a, b),
            Arith3264Op::Add64(a, b) => self.arith64_sm.add(a, b),
            Arith3264Op::Sub32(a, b) => self.arith32_sm.sub(a, b),
            Arith3264Op::Sub64(a, b) => self.arith64_sm.sub(a, b),
        }
    }

    fn prove(&self, operations: &[Arith3264Op]) {
        let mut inputs32 = self.inputs32.lock().unwrap();
        let mut inputs64 = self.inputs64.lock().unwrap();
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

        if inputs32.len() >= PROVE_CHUNK_SIZE {
            let old_inputs = std::mem::take(&mut *inputs32);
            self.arith32_sm.prove(&old_inputs);
        }

        if inputs64.len() >= PROVE_CHUNK_SIZE {
            let old_inputs = std::mem::take(&mut *inputs64);
            self.arith64_sm.prove(&old_inputs);
        }
    }

    fn calculate_prove(
        &self,
        operation: Arith3264Op,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for ArithSM {
    fn when_closed(&self) {
        if let Ok(mut inputs) = self.inputs32.lock() {
            if !inputs.is_empty() {
                let old_inputs = std::mem::take(&mut *inputs);
                self.arith32_sm.prove(&old_inputs);
            }
        }

        if let Ok(mut inputs) = self.inputs64.lock() {
            if !inputs.is_empty() {
                let old_inputs = std::mem::take(&mut *inputs);
                self.arith64_sm.prove(&old_inputs);
            }
        }

        // Close open sessions for the current thread
        for session_id in &self.opened_sessions {
            self.sessions.close_session(*session_id).expect("Failed to close session");
        }
    }
}
