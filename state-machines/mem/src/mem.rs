use log::debug;
use rayon::Scope;
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
}

impl MemSM {
    pub fn new<F>(
        wcm: &mut WCManager<F>,
        mem_aligned_sm: Arc<MemAlignedSM>,
        mem_unaligned_sm: Arc<MemUnalignedSM>,
    ) -> Arc<Self> {
        let mem_sm = Self {
            inputs_aligned: Mutex::new(Vec::new()),
            inputs_unaligned: Mutex::new(Vec::new()),
            mem_aligned_sm,
            mem_unaligned_sm,
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

    fn prove(&self, operations: &[MemOp], is_last: bool, scope: &Scope) {
        let mut inputs_aligned = self.inputs_aligned.lock().unwrap();
        let mut inputs_unaligned = self.inputs_unaligned.lock().unwrap();

        // TODO! Split the operations into 32 and 64 bit operations in parallel
        for operation in operations {
            match operation {
                MemOp::Read(addr) => {
                    if addr % 8 == 0 {
                        inputs_aligned.push(operation.clone());
                    } else {
                        let width = 8; // TODO!
                        inputs_unaligned.push(sm_common::MemUnalignedOp::Read(*addr, width))
                    }
                }
                MemOp::Write(addr, val) => {
                    if addr % 8 == 0 {
                        inputs_aligned.push(operation.clone());
                    } else {
                        let width = 8; // TODO!
                        inputs_unaligned.push(sm_common::MemUnalignedOp::Write(*addr, width, *val))
                    }
                }
            }
        }

        // The following is a way to release the lock on the inputs32 and inputs64 Mutexes asap
        // NOTE: The `inputs32` lock is released when it goes out of scope because it is shadowed
        let inputs_aligned = if is_last || inputs_aligned.len() >= PROVE_CHUNK_SIZE {
            let _inputs_aligned = std::mem::take(&mut *inputs_aligned);
            if _inputs_aligned.is_empty() {
                None
            } else {
                Some(_inputs_aligned)
            }
        } else {
            None
        };

        // NOTE: The `inputs64` lock is released when it goes out of scope because it is shadowed
        let inputs_unaligned = if is_last || inputs_unaligned.len() >= PROVE_CHUNK_SIZE {
            let _inputs_unaligned = std::mem::take(&mut *inputs_unaligned);
            if _inputs_unaligned.is_empty() {
                None
            } else {
                Some(_inputs_unaligned)
            }
        } else {
            None
        };

        if inputs_aligned.is_some() {
            let mem_aligned_sm = self.mem_aligned_sm.clone();
            scope.spawn(move |scope| {
                mem_aligned_sm.prove(&inputs_aligned.unwrap(), is_last, scope);
            });
        }

        if inputs_unaligned.is_some() {
            let mem_unaligned_sm: Arc<MemUnalignedSM> = self.mem_unaligned_sm.clone();
            scope.spawn(move |scope| {
                mem_unaligned_sm.prove(&inputs_unaligned.unwrap(), is_last, scope);
            });
        }
    }

    fn calculate_prove(
        &self,
        operation: MemOp,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
