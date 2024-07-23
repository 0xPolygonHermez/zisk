use log::debug;
use sm_common::{MemOp, OpResult, Provable, Sessionable, Sessions};
use sm_mem_aligned::MemAlignedSM;
use sm_mem_unaligned::MemUnalignedSM;
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCOpCalculator};

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct MemSM {
    inputs_aligned: Arc<RwLock<Vec<MemOp>>>,
    inputs_unaligned: Arc<RwLock<Vec<MemOp>>>,
    last_proved_aligned_idx: AtomicUsize,
    last_proved_unaligned_idx: AtomicUsize,
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
        let inputs_aligned = Arc::new(RwLock::new(Vec::new()));
        let inputs_unaligned = Arc::new(RwLock::new(Vec::new()));

        let opened_sessions = vec![
            sessions.open_session(mem_aligned_sm.clone()),
            sessions.open_session(mem_unaligned_sm.clone()),
        ];

        let mem_sm = Self {
            inputs_aligned,
            inputs_unaligned,
            last_proved_aligned_idx: AtomicUsize::new(0),
            last_proved_unaligned_idx: AtomicUsize::new(0),
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
                    self.mem_unaligned_sm.calculate(operation)
                }
            }
            MemOp::Write(addr, val) => {
                if addr % 8 == 0 {
                    self.mem_aligned_sm.calculate(operation)
                } else {
                    self.mem_unaligned_sm.calculate(operation)
                }
            }
        }
    }

    fn prove(&self, operations: &[MemOp]) {
        // Create a scoped block to hold the write lock only the necessary
        let (num_inputs_aligned, num_inputs_unaligned) = {
            let mut inputs_aligned = self.inputs_aligned.write().unwrap();
            let mut inputs_unaligned = self.inputs_unaligned.write().unwrap();
            for operation in operations {
                match operation {
                    MemOp::Read(addr) => {
                        if addr % 8 == 0 {
                            inputs_aligned.push(operation.clone());
                        } else {
                            inputs_unaligned.push(operation.clone());
                        }
                    }
                    MemOp::Write(addr, val) => {
                        if addr % 8 == 0 {
                            inputs_aligned.push(operation.clone());
                        } else {
                            inputs_unaligned.push(operation.clone());
                        }
                    }
                }
            }
            (inputs_aligned.len(), inputs_unaligned.len())
        };

        if num_inputs_aligned % PROVE_CHUNK_SIZE == 0 {
            let last_proved_aligned_idx = self.last_proved_aligned_idx.load(Ordering::Relaxed);
            self.mem_aligned_sm.prove(
                &self.inputs_aligned.read().unwrap()[last_proved_aligned_idx..num_inputs_aligned],
            );
            self.last_proved_aligned_idx.store(num_inputs_aligned, Ordering::Relaxed);
        }

        if num_inputs_unaligned % PROVE_CHUNK_SIZE == 0 {
            let last_proved_unaligned_idx = self.last_proved_unaligned_idx.load(Ordering::Relaxed);
            self.mem_unaligned_sm.prove(
                &self.inputs_unaligned.read().unwrap()
                    [last_proved_unaligned_idx..num_inputs_unaligned],
            );
            self.last_proved_unaligned_idx.store(num_inputs_unaligned, Ordering::Relaxed);
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
        // Prove remaining inputs if any
        // TODO We need to prove the remaining inputs. If the number of inputs32 and inputs64 fits
        // in a single proof, we can prove them together using 3264.
        let num_inputs_aligned = { self.inputs_aligned.read().unwrap().len() };
        let last_proved_aligned_idx = self.last_proved_aligned_idx.load(Ordering::Relaxed);
        self.mem_aligned_sm.prove(
            &self.inputs_aligned.read().unwrap()[last_proved_aligned_idx..num_inputs_aligned],
        );

        let num_inputs_unaligned = { self.inputs_unaligned.read().unwrap().len() };
        let last_proved_unaligned_idx = self.last_proved_unaligned_idx.load(Ordering::Relaxed);
        self.mem_unaligned_sm.prove(
            &self.inputs_unaligned.read().unwrap()[last_proved_unaligned_idx..num_inputs_unaligned],
        );

        // Close open sessions for the current thread
        for session_id in &self.opened_sessions {
            self.sessions.close_session(*session_id).expect("Failed to close session");
        }
    }
}
