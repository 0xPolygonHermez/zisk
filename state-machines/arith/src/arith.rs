use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};

use crate::{Arith32SM, ArithFullSM, ArithMul32SM, ArithMul64SM, ArithRangeTableSM, ArithTableSM};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct ArithSM<F> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
    inputs_32: Mutex<Vec<ZiskRequiredOperation>>,
    inputs_mul_32: Mutex<Vec<ZiskRequiredOperation>>,
    inputs_mul_64: Mutex<Vec<ZiskRequiredOperation>>,

    // Secondary State machines
    arith_32_sm: Arc<Arith32SM<F>>,
    arith_mul_32_sm: Arc<ArithMul32SM<F>>,
    arith_mul_64_sm: Arc<ArithMul64SM<F>>,
    arith_full_sm: Arc<ArithFullSM<F>>,
    arith_range_table_sm: Arc<ArithRangeTableSM<F>>,
    arith_table_sm: Arc<ArithTableSM<F>>,
}

impl<F: Field> ArithSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        // TODO: change this call, for calls to WitnessManager to obtain from airGroupId and airIds
        // ON each SM, not need pass to the constructor
        let arith_full_ids = ArithSM::<F>::get_ids_by_name("Arith");
        let arith_32_ids = ArithSM::<F>::get_ids_by_name("Arith32");
        let arith_mul_32_ids = ArithSM::<F>::get_ids_by_name("ArithMul32");
        let arith_mul_64_ids = ArithSM::<F>::get_ids_by_name("ArithMul64");
        let arith_range_table_ids = ArithSM::<F>::get_ids_by_name("ArithRangeTable");
        let arith_table_ids = ArithSM::<F>::get_ids_by_name("ArithTable");

        let arith_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs: Mutex::new(Vec::new()),
            inputs_32: Mutex::new(Vec::new()),
            inputs_mul_32: Mutex::new(Vec::new()),
            inputs_mul_64: Mutex::new(Vec::new()),
            arith_full_sm: ArithFullSM::new(wcm.clone(), arith_full_ids.0, &[arith_full_ids.1]),
            arith_32_sm: Arith32SM::new(wcm.clone(), arith_32_ids.0, &[arith_32_ids.1]),
            arith_mul_32_sm: ArithMul32SM::new(
                wcm.clone(),
                arith_mul_32_ids.0,
                &[arith_mul_32_ids.1],
            ),
            arith_mul_64_sm: ArithMul64SM::new(
                wcm.clone(),
                arith_mul_64_ids.0,
                &[arith_mul_64_ids.1],
            ),
            arith_range_table_sm: ArithRangeTableSM::new(
                wcm.clone(),
                arith_range_table_ids.0,
                &[arith_range_table_ids.1],
            ),
            arith_table_sm: ArithTableSM::new(wcm.clone(), arith_table_ids.0, &[arith_table_ids.1]),
        };
        let arith_sm = Arc::new(arith_sm);

        wcm.register_component(arith_sm.clone(), None, None);

        arith_sm.arith_32_sm.register_predecessor();
        arith_sm.arith_mul_32_sm.register_predecessor();
        arith_sm.arith_mul_64_sm.register_predecessor();
        arith_sm.arith_full_sm.register_predecessor();

        arith_sm
    }

    pub fn get_ids_by_name(name: &str) -> (usize, usize) {
        const ARITH_AIRGROUP_ID: usize = 1;
        if name == "Arith" {
            return (ARITH_AIRGROUP_ID, 10);
        } else if name == "Arith32" {
            return (ARITH_AIRGROUP_ID, 11);
        } else if name == "ArithMul64" {
            return (ARITH_AIRGROUP_ID, 12);
        } else if name == "ArithMul32" {
            return (ARITH_AIRGROUP_ID, 13);
        } else if name == "AirthRangeTable" {
            return (ARITH_AIRGROUP_ID, 14);
        } else if name == "ArithTable" {
            return (ARITH_AIRGROUP_ID, 15);
        }
        return (0, 0);
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <ArithSM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );

            self.threads_controller.wait_for_threads();

            self.arith_32_sm.unregister_predecessor(scope);
            self.arith_mul_32_sm.unregister_predecessor(scope);
            self.arith_mul_64_sm.unregister_predecessor(scope);
            self.arith_full_sm.unregister_predecessor(scope);
        }
    }
}

impl<F: Field> WitnessComponent<F> for ArithSM<F> {
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

impl<F: Field> Provable<ZiskRequiredOperation, OpResult> for ArithSM<F> {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = ZiskOp::execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        let mut _inputs32 = Vec::new();
        let mut _inputs64 = Vec::new();

        let operations64 = ArithMul64SM::<F>::operations();
        let operations32 = Arith32SM::<F>::operations();

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
        /*
        let mut inputs32 = self.inputs_32.lock().unwrap();
        inputs32.extend(_inputs32);

        while inputs32.len() >= PROVE_CHUNK_SIZE || (drain && !inputs32.is_empty()) {
            if drain && !inputs32.is_empty() {
                // println!("ArithSM: Draining inputs32");
            }

            let num_drained32 = std::cmp::min(PROVE_CHUNK_SIZE, inputs32.len());
            let drained_inputs32 = inputs32.drain(..num_drained32).collect::<Vec<_>>();
            let arith32_sm_cloned = self.arith_32_sm.clone();

            self.threads_controller.add_working_thread();
            let thread_controller = self.threads_controller.clone();

            scope.spawn(move |scope| {
                arith32_sm_cloned.prove(&drained_inputs32, drain, scope);

                thread_controller.remove_working_thread();
            });
        }
        drop(inputs32);

        let mut inputs64 = self.inputs_mul_64.lock().unwrap();
        inputs64.extend(_inputs64);

        while inputs64.len() >= PROVE_CHUNK_SIZE || (drain && !inputs64.is_empty()) {
            if drain && !inputs64.is_empty() {
                // println!("ArithSM: Draining inputs64");
            }

            let num_drained64 = std::cmp::min(PROVE_CHUNK_SIZE, inputs64.len());
            let drained_inputs64 = inputs64.drain(..num_drained64).collect::<Vec<_>>();
            let arith64_sm_cloned = self.arith_mul_64_sm.clone();

            self.threads_controller.add_working_thread();
            let thread_controller = self.threads_controller.clone();

            scope.spawn(move |scope| {
                arith64_sm_cloned.prove(&drained_inputs64, drain, scope);

                thread_controller.remove_working_thread();
            });
        }
        drop(inputs64);*/
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
