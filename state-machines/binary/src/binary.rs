use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{fmt::Debug, sync::mpsc, thread};

use proofman::WCManager;
use sm_binary_32::Binary32SM;
use sm_binary_3264::Binary3264SM;
use sm_binary_64::Binary64SM;
use sm_common::{
    Binary3264Op, Binary32Op, Binary64Op, OpResult, Provable, Sessionable, Sessions, WorkerHandler,
    WorkerTask,
};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct BinarySM {
    inputs32: Mutex<Vec<Binary32Op>>,
    inputs64: Mutex<Vec<Binary64Op>>,
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
        let opened_sessions = vec![
            sessions.open_session(binary32_sm.clone()),
            sessions.open_session(binary64_sm.clone()),
            sessions.open_session(binary3264_sm.clone()),
        ];

        let binary_sm = Self {
            inputs32: Mutex::new(Vec::new()),
            inputs64: Mutex::new(Vec::new()),
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

impl Provable<Binary3264Op, OpResult> for BinarySM {
    fn calculate(&self, operation: Binary3264Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Binary3264Op::And32(a, b) => self.binary32_sm.and(a, b),
            Binary3264Op::And64(a, b) => self.binary64_sm.and(a, b),
            Binary3264Op::Or32(a, b) => self.binary32_sm.or(a, b),
            Binary3264Op::Or64(a, b) => self.binary64_sm.or(a, b),
        }
    }

    fn prove(&self, operations: &[Binary3264Op]) {
        let mut inputs32 = self.inputs32.lock().unwrap();
        let mut inputs64 = self.inputs64.lock().unwrap();
        for operation in operations {
            match operation {
                Binary3264Op::And32(a, b) | Binary3264Op::Or32(a, b) => {
                    inputs32.push(operation.clone().into());
                }
                Binary3264Op::And64(a, b) | Binary3264Op::Or64(a, b) => {
                    inputs64.push(operation.clone().into());
                }
            }
        }

        if inputs32.len() >= PROVE_CHUNK_SIZE {
            let old_inputs = std::mem::take(&mut *inputs32);
            self.binary32_sm.prove(&old_inputs);
        }

        if inputs64.len() >= PROVE_CHUNK_SIZE {
            let old_inputs = std::mem::take(&mut *inputs64);
            self.binary64_sm.prove(&old_inputs);
        }
    }

    fn calculate_prove(
        &self,
        operation: Binary3264Op,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for BinarySM {
    fn when_closed(&self) {
        if let Ok(mut inputs) = self.inputs32.lock() {
            if !inputs.is_empty() {
                let old_inputs = std::mem::take(&mut *inputs);
                self.binary32_sm.prove(&old_inputs);
            }
        }

        if let Ok(mut inputs) = self.inputs64.lock() {
            if !inputs.is_empty() {
                let old_inputs = std::mem::take(&mut *inputs);
                self.binary64_sm.prove(&old_inputs);
            }
        }

        // Close open sessions for the current thread
        for session_id in &self.opened_sessions {
            self.sessions.close_session(*session_id).expect("Failed to close session");
        }
    }
}
