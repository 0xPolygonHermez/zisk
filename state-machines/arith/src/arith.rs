use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{Arith3264SM, Arith32SM, Arith64SM};
use p3_field::AbstractField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct ArithSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

    // Inputs
    inputs32: Mutex<Vec<ZiskRequiredOperation>>,
    inputs64: Mutex<Vec<ZiskRequiredOperation>>,

    // Secondary State machines
    arith32_sm: Arc<Arith32SM>,
    arith64_sm: Arc<Arith64SM>,
    arith3264_sm: Arc<Arith3264SM>,
}

impl ArithSM {
    pub fn new<F>(
        wcm: &mut WitnessManager<F>,
        arith32_sm: Arc<Arith32SM>,
        arith64_sm: Arc<Arith64SM>,
        arith3264_sm: Arc<Arith3264SM>,
    ) -> Arc<Self> {
        let arith_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs32: Mutex::new(Vec::new()),
            inputs64: Mutex::new(Vec::new()),
            arith32_sm,
            arith64_sm,
            arith3264_sm,
        };
        let arith_sm = Arc::new(arith_sm);

        wcm.register_component(arith_sm.clone(), None, None);

        arith_sm.arith32_sm.register_predecessor();
        arith_sm.arith64_sm.register_predecessor();
        arith_sm.arith3264_sm.register_predecessor();

        arith_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor<F: AbstractField>(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <ArithSM as Provable<ZiskRequiredOperation, OpResult>>::prove(self, &[], true, scope);

            self.threads_controller.wait_for_threads();

            self.arith3264_sm.unregister_predecessor::<F>(scope);
            self.arith64_sm.unregister_predecessor::<F>(scope);
            self.arith32_sm.unregister_predecessor::<F>(scope);
        }
    }
}

impl<F> WitnessComponent<F> for ArithSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for ArithSM {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        let mut _inputs32 = Vec::new();
        let mut _inputs64 = Vec::new();

        let operations32 = Arith32SM::operations();
        let operations64 = Arith64SM::operations();

        // TODO Split the operations into 32 and 64 bit operations in parallel
        for operation in operations {
            if operations32.contains(&operation.opcode) {
                _inputs32.push(operation.clone());
            } else if operations64.contains(&operation.opcode) {
                _inputs64.push(operation.clone());
            } else {
                panic!("ArithSM: Operator {:x} not found", operation.opcode);
            }
        }

        // TODO When drain is true, drain remaining inputs to the 3264 bits state machine

        let mut inputs32 = self.inputs32.lock().unwrap();
        inputs32.extend(_inputs32);

        while inputs32.len() >= PROVE_CHUNK_SIZE || (drain && !inputs32.is_empty()) {
            if drain && !inputs32.is_empty() {
                println!("ArithSM: Draining inputs32");
            }

            let num_drained32 = std::cmp::min(PROVE_CHUNK_SIZE, inputs32.len());
            let drained_inputs32 = inputs32.drain(..num_drained32).collect::<Vec<_>>();
            let arith32_sm_cloned = self.arith32_sm.clone();

            self.threads_controller.add_working_thread();
            let thread_controller = self.threads_controller.clone();

            scope.spawn(move |scope| {
                arith32_sm_cloned.prove(&drained_inputs32, drain, scope);

                thread_controller.remove_working_thread();
            });
        }
        drop(inputs32);

        let mut inputs64 = self.inputs64.lock().unwrap();
        inputs64.extend(_inputs64);

        while inputs64.len() >= PROVE_CHUNK_SIZE || (drain && !inputs64.is_empty()) {
            if drain && !inputs64.is_empty() {
                println!("ArithSM: Draining inputs64");
            }

            let num_drained64 = std::cmp::min(PROVE_CHUNK_SIZE, inputs64.len());
            let drained_inputs64 = inputs64.drain(..num_drained64).collect::<Vec<_>>();
            let arith64_sm_cloned = self.arith64_sm.clone();

            self.threads_controller.add_working_thread();
            let thread_controller = self.threads_controller.clone();

            scope.spawn(move |scope| {
                arith64_sm_cloned.prove(&drained_inputs64, drain, scope);

                thread_controller.remove_working_thread();
            });
        }
        drop(inputs64);
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredOperation,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());

        self.prove(&[operation], drain, scope);

        result
    }
}
