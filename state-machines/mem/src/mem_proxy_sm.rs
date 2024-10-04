use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{MemAlignSM, MemSM};
use p3_field::Field;
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use zisk_core::ZiskRequiredMemory;

use proofman::{WitnessComponent, WitnessManager};

const PROVE_CHUNK_SIZE: usize = 1 << 16;

#[allow(dead_code)]
pub struct MemProxySM<F> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

    // Inputs
    inputs_aligned: Mutex<Vec<ZiskRequiredMemory>>,
    inputs_unaligned: Mutex<Vec<ZiskRequiredMemory>>,

    // Secondary State machines
    mem_sm: Arc<MemSM<F>>,
    mem_align_sm: Arc<MemAlignSM>,
}

impl<F: Field> MemProxySM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_sm = MemSM::new(wcm.clone());
        let mem_align_sm = MemAlignSM::new(wcm.clone());

        let mem_proxy_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs_aligned: Mutex::new(Vec::new()),
            inputs_unaligned: Mutex::new(Vec::new()),
            mem_sm,
            mem_align_sm,
        };
        let mem_proxy_sm = Arc::new(mem_proxy_sm);

        wcm.register_component(mem_proxy_sm.clone(), None, None);

        // For all the secondary state machines, register the mem proxy as a predecessor
        mem_proxy_sm.mem_sm.register_predecessor();
        mem_proxy_sm.mem_align_sm.register_predecessor();

        mem_proxy_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <MemProxySM<F> as Provable<ZiskRequiredMemory, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );

            self.threads_controller.wait_for_threads();

            self.mem_sm.unregister_predecessor(scope);
            self.mem_align_sm.unregister_predecessor::<F>(scope);
        }
    }
}

impl<F: Field> WitnessComponent<F> for MemProxySM<F> {}

impl<F: Field> Provable<ZiskRequiredMemory, OpResult> for MemProxySM<F> {
    fn prove(&self, operations: &[ZiskRequiredMemory], drain: bool, scope: &Scope) {
        let mut _inputs_aligned = Vec::new();
        let mut _inputs_unaligned = Vec::new();

        // Classify the operations into aligned and unaligned
        // TODO Do it in parallel
        for operation in operations {
            let is_aligned = operation.address % 8 == 0;
            if is_aligned {
                _inputs_aligned.push(operation.clone());
            } else {
                _inputs_unaligned.push(operation.clone());
            }
        }

        let mut inputs_aligned = self.inputs_aligned.lock().unwrap();
        inputs_aligned.extend(_inputs_aligned);

        while inputs_aligned.len() >= PROVE_CHUNK_SIZE || (drain && !inputs_aligned.is_empty()) {
            let num_drained_aligned = std::cmp::min(PROVE_CHUNK_SIZE, inputs_aligned.len());
            let drained_inputs_aligned = inputs_aligned.drain(..num_drained_aligned).collect::<Vec<_>>();

            let mem_sm_cloned = self.mem_sm.clone();

            self.threads_controller.add_working_thread();
            let thread_controller = self.threads_controller.clone();

            scope.spawn(move |scope| {
                mem_sm_cloned.prove(&drained_inputs_aligned, false, scope);

                thread_controller.remove_working_thread();
            });
        }
        drop(inputs_aligned);

            let mut inputs_unaligned = self.inputs_aligned.lock().unwrap();
        inputs_unaligned.extend(_inputs_unaligned);

        while inputs_unaligned.len() >= PROVE_CHUNK_SIZE || (drain && !inputs_unaligned.is_empty()) {
            let num_drained_unaligned = std::cmp::min(PROVE_CHUNK_SIZE, inputs_unaligned.len());
            let drained_inputs_unaligned = inputs_unaligned.drain(..num_drained_unaligned).collect::<Vec<_>>();

            let mem_align_sm_cloned = self.mem_align_sm.clone();

            self.threads_controller.add_working_thread();
            let thread_controller = self.threads_controller.clone();

            scope.spawn(move |scope| {
                mem_align_sm_cloned.prove(&drained_inputs_unaligned, false, scope);

                thread_controller.remove_working_thread();
            });
        }
        drop(inputs_unaligned);

    }
}
