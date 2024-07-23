use log::debug;
use sm_common::{MemOp, MemUnalignedOp, OpResult, Provable, Sessionable, Sessions};
use sm_mem_aligned::MemAlignedSM;
use sm_mem_unaligned::MemUnalignedSM;
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCOpCalculator};

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct MemSM {
    inputs_aligned: Mutex<Vec<MemOp>>,
    inputs_unaligned: Mutex<Vec<MemUnalignedOp>>,
    mem_aligned_sm: Arc<MemAlignedSM>,
    mem_unaligned_sm: Arc<MemUnalignedSM>,
    sessions: Arc<Sessions>,
    opened_sessions: Vec<usize>,
}

impl MemSM {
    pub fn new<F>(
        wcm: &mut WCManager<F>,
        sessions: Arc<Sessions>,
        mem_aligned_sm: Arc<MemAlignedSM>,
        mem_unaligned_sm: Arc<MemUnalignedSM>,
    ) -> Arc<Self> {
        let opened_sessions = vec![
            sessions.open_session(mem_aligned_sm.clone()),
            sessions.open_session(mem_unaligned_sm.clone()),
        ];

        let mem_sm = Self {
            inputs_aligned: Mutex::new(Vec::new()),
            inputs_unaligned: Mutex::new(Vec::new()),
            mem_aligned_sm,
            mem_unaligned_sm,
            sessions,
            opened_sessions,
        };
        let mem_sm = Arc::new(mem_sm);

        wcm.register_component(mem_sm.clone() as Arc<dyn WCComponent<F>>, None);

        mem_sm
    }
}

impl<F> WCComponent<F> for MemSM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }

    fn suggest_plan(&self, _ectx: &mut ExecutionCtx) {}
}

impl Provable<MemOp, OpResult> for MemSM {
    fn calculate(&self, operation: MemOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            MemOp::Read(addr) => {
                if addr % 8 == 0 {
                    self.mem_aligned_sm.calculate(operation)
                } else {
                    let width = 8; // TODO!
                    self.mem_unaligned_sm.calculate(sm_common::MemUnalignedOp::Read(addr, width))
                }
            }
            MemOp::Write(addr, val) => {
                if addr % 8 == 0 {
                    self.mem_aligned_sm.calculate(operation)
                } else {
                    let width = 8; // TODO!
                    self.mem_unaligned_sm
                        .calculate(sm_common::MemUnalignedOp::Write(addr, width, val))
                }
            }
        }
    }

    fn prove(&self, operations: &[MemOp]) {
        let mut inputs_aligned = self.inputs_aligned.lock().unwrap();
        let mut inputs_unaligned = self.inputs_unaligned.lock().unwrap();
        for operation in operations {
            match operation {
                MemOp::Read(addr) => {
                    if addr % 8 == 0 {
                        inputs_aligned.push(operation.clone());
                    } else {
                        let width = 8; // TODO!
                        inputs_unaligned.push(sm_common::MemUnalignedOp::Read(*addr, width));
                    }
                }
                MemOp::Write(addr, val) => {
                    if addr % 8 == 0 {
                        inputs_aligned.push(operation.clone());
                    } else {
                        let width = 8; // TODO!
                        inputs_unaligned.push(sm_common::MemUnalignedOp::Write(*addr, width, *val));
                    }
                }
            }
        }

        if inputs_aligned.len() >= PROVE_CHUNK_SIZE {
            let old_inputs = std::mem::take(&mut *inputs_aligned);
            self.mem_aligned_sm.prove(&old_inputs);
        }

        if inputs_unaligned.len() >= PROVE_CHUNK_SIZE {
            let old_inputs = std::mem::take(&mut *inputs_unaligned);
            self.mem_unaligned_sm.prove(&old_inputs);
        }
    }

    fn calculate_prove(&self, operation: MemOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for MemSM {
    fn when_closed(&self) {
        if let Ok(mut inputs) = self.inputs_aligned.lock() {
            if !inputs.is_empty() {
                let old_inputs = std::mem::take(&mut *inputs);
                self.mem_aligned_sm.prove(&old_inputs);
            }
        }

        if let Ok(mut inputs) = self.inputs_unaligned.lock() {
            if !inputs.is_empty() {
                let old_inputs = std::mem::take(&mut *inputs);
                self.mem_unaligned_sm.prove(&old_inputs);
            }
        }

        // Close open sessions for the current thread
        for session_id in &self.opened_sessions {
            self.sessions.close_session(*session_id).expect("Failed to close session");
        }
    }
}
