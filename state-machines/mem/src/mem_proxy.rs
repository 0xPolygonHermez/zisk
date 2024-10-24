use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::PrimeField;
use rayon::Scope;
use sm_common::{MemOp, MemUnalignedOp, OpResult, Provable, ThreadController};
use zisk_core::ZiskRequiredMemory;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use crate::MemSM;

#[allow(dead_code)]
const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct MemProxy<F: PrimeField> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

    // Inputs
    inputs_aligned: Mutex<Vec<MemOp>>,
    inputs_unaligned: Mutex<Vec<MemUnalignedOp>>,

    // Secondary State machines
    mem_sm: Arc<MemSM<F>>,
}

impl<F: PrimeField> MemProxy<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_sm = MemSM::new(wcm.clone());

        let mem_proxy = Self {
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs_aligned: Mutex::new(Vec::new()),
            inputs_unaligned: Mutex::new(Vec::new()),
            mem_sm: mem_sm.clone(),
        };
        let mem_proxy = Arc::new(mem_proxy);

        wcm.register_component(mem_proxy.clone(), None, None);

        // For all the secondary state machines, register the main state machine as a predecessor
        mem_proxy.mem_sm.register_predecessor();

        mem_proxy
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <MemProxy<F> as Provable<ZiskRequiredMemory, OpResult>>::prove(self, &[], true, scope);

            self.threads_controller.remove_working_thread();

            self.mem_sm.unregister_predecessor(scope);
        }
    }

    pub fn prove_instance(
        &self,
        mem_ops: &[ZiskRequiredMemory],
        mem_first_row: ZiskRequiredMemory,
        segment_id: usize,
        is_last_segment: bool,
        prover_buffer: Vec<F>,
        offset: u64,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.mem_sm.prove_instance(
            mem_ops,
            mem_first_row,
            segment_id,
            is_last_segment,
            prover_buffer,
            offset,
            pctx,
            ectx,
            sctx,
        ).expect("Failed to prove mem instance");
    }
    //pub fn prove_instance<F: Field>(
}

impl<F: PrimeField> WitnessComponent<F> for MemProxy<F> {}

impl<F: PrimeField> Provable<ZiskRequiredMemory, OpResult> for MemProxy<F> {
    fn prove(&self, _operations: &[ZiskRequiredMemory], _drain: bool, _scope: &Scope) {
        // TODO!
    }
}
