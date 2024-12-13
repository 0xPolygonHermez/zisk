use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{MemAlignedSM, MemUnalignedSM};
use p3_field::Field;
use rayon::Scope;
use sm_common::{MemOp, MemUnalignedOp, OpResult, Provable};
use zisk_core::ZiskRequiredMemory;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ProofCtx, SetupCtx};

#[allow(dead_code)]
const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct MemSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs_aligned: Mutex<Vec<MemOp>>,
    inputs_unaligned: Mutex<Vec<MemUnalignedOp>>,

    // Secondary State machines
    mem_aligned_sm: Arc<MemAlignedSM>,
    mem_unaligned_sm: Arc<MemUnalignedSM>,
}

impl MemSM {
    pub fn new<F>(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_aligned_sm = MemAlignedSM::new(wcm.clone());
        let mem_unaligned_sm = MemUnalignedSM::new(wcm.clone());

        let mem_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            inputs_aligned: Mutex::new(Vec::new()),
            inputs_unaligned: Mutex::new(Vec::new()),
            mem_aligned_sm: mem_aligned_sm.clone(),
            mem_unaligned_sm: mem_unaligned_sm.clone(),
        };
        let mem_sm = Arc::new(mem_sm);

        wcm.register_proxy_component(mem_sm.clone());

        // For all the secondary state machines, register the main state machine as a predecessor
        mem_sm.mem_aligned_sm.register_predecessor();
        mem_sm.mem_unaligned_sm.register_predecessor();

        mem_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor<F: Field>(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <MemSM as Provable<ZiskRequiredMemory, OpResult>>::prove(self, &[], true, scope);

            self.mem_aligned_sm.unregister_predecessor::<F>(scope);
            self.mem_unaligned_sm.unregister_predecessor::<F>(scope);
        }
    }
}

impl<F> WitnessComponent<F> for MemSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}

impl Provable<ZiskRequiredMemory, OpResult> for MemSM {
    fn calculate(
        &self,
        _operation: ZiskRequiredMemory,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        unimplemented!()
    }

    fn prove(&self, _operations: &[ZiskRequiredMemory], _drain: bool, _scope: &Scope) {
        // TODO!
    }

    fn calculate_prove(
        &self,
        _operation: ZiskRequiredMemory,
        _drain: bool,
        _scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        unimplemented!()
    }
}
