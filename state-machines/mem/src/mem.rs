use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Condvar, Mutex,
};

use crate::{MemAlignedSM, MemUnalignedSM};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{MemOp, MemUnalignedOp, OpResult, Provable};
use zisk_core::ZiskRequiredMemory;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};

#[allow(dead_code)]
const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct MemSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Mechanism to control the number of working threads
    working_threads: Arc<AtomicU32>,
    mutex: Arc<Mutex<()>>,
    condvar: Arc<Condvar>,

    // Inputs
    inputs_aligned: Mutex<Vec<MemOp>>,
    inputs_unaligned: Mutex<Vec<MemUnalignedOp>>,

    // Secondary State machines
    mem_aligned_sm: Arc<MemAlignedSM>,
    mem_unaligned_sm: Arc<MemUnalignedSM>,
}

impl MemSM {
    pub fn new<F>(
        wcm: &mut WitnessManager<F>,
        mem_aligned_sm: Arc<MemAlignedSM>,
        mem_unaligned_sm: Arc<MemUnalignedSM>,
    ) -> Arc<Self> {
        let mem_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            working_threads: Arc::new(AtomicU32::new(0)),
            mutex: Arc::new(Mutex::new(())),
            condvar: Arc::new(Condvar::new()),
            inputs_aligned: Mutex::new(Vec::new()),
            inputs_unaligned: Mutex::new(Vec::new()),
            mem_aligned_sm: mem_aligned_sm.clone(),
            mem_unaligned_sm: mem_unaligned_sm.clone(),
        };
        let mem_sm = Arc::new(mem_sm);

        wcm.register_component(mem_sm.clone(), None);

        // For all the secondary state machines, register the main state machine as a predecessor
        mem_sm.mem_aligned_sm.register_predecessor();
        mem_sm.mem_unaligned_sm.register_predecessor();

        mem_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <MemSM as Provable<ZiskRequiredMemory, OpResult>>::prove(self, &[], true, scope);

            let mut guard = self.mutex.lock().unwrap();
            while self.working_threads.load(Ordering::SeqCst) > 0 {
                guard = self.condvar.wait(guard).unwrap();
            }

            self.mem_aligned_sm.unregister_predecessor(scope);
            self.mem_unaligned_sm.unregister_predecessor(scope);
        }
    }
}

impl<F> WitnessComponent<F> for MemSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }
}

impl Provable<ZiskRequiredMemory, OpResult> for MemSM {
    /*fn calculate(
        &self,
        _operation: ZiskRequiredMemory,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        unimplemented!()
    }*/

    fn prove(&self, _operations: &[ZiskRequiredMemory], _drain: bool, _scope: &Scope) {
        // TODO!
    }

    /*fn calculate_prove(
        &self,
        _operation: ZiskRequiredMemory,
        _drain: bool,
        _scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        unimplemented!()
    }*/
}
