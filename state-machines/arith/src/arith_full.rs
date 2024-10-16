use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{
    arith_table_inputs, ArithRangeTableInputs, ArithRangeTableSM, ArithSM, ArithTableInputs,
    ArithTableSM,
};
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::Arith0Row;

const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct ArithFullSM<F> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
    arith_table_sm: Arc<ArithTableSM<F>>,
    arith_range_table_sm: Arc<ArithRangeTableSM<F>>,
}

impl<F: Field> ArithFullSM<F> {
    const MY_NAME: &'static str = "Arith   ";
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        arith_table_sm: Arc<ArithTableSM<F>>,
        arith_range_table_sm: Arc<ArithRangeTableSM<F>>,
        airgroup_id: usize,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let arith_full_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs: Mutex::new(Vec::new()),
            arith_table_sm,
            arith_range_table_sm,
        };
        let arith_full_sm = Arc::new(arith_full_sm);

        wcm.register_component(arith_full_sm.clone(), Some(airgroup_id), Some(air_ids));

        arith_full_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <ArithFullSM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
            self.threads_controller.wait_for_threads();

            self.arith_table_sm.unregister_predecessor(scope);
            self.arith_range_table_sm.unregister_predecessor(scope);
        }
    }
    pub fn process_slice(
        input: &Vec<ZiskRequiredOperation>,
        range_table_inputs: &mut ArithRangeTableInputs<F>,
        table_inputs: &mut ArithTableInputs<F>,
    ) -> Vec<Arith0Row<F>> {
        let mut _trace: Vec<Arith0Row<F>> = Vec::new();
        range_table_inputs.push(0, 0);
        table_inputs.fast_push(0, 0, 0);
        _trace
    }
}

impl<F: Field> WitnessComponent<F> for ArithFullSM<F> {
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

impl<F: Field> Provable<ZiskRequiredOperation, OpResult> for ArithFullSM<F> {
    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                if drain && !inputs.is_empty() {
                    println!("ArithFullSM: Draining inputs");
                }

                // self.threads_controller.add_working_thread();
                // let thread_controller = self.threads_controller.clone();

                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let _drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                scope.spawn(move |_| {
                    let mut arith_range_table_inputs = ArithRangeTableInputs::<F>::new();
                    let mut arith_table_inputs = ArithTableInputs::<F>::new();
                    let _trace = Self::process_slice(
                        &_drained_inputs,
                        &mut arith_range_table_inputs,
                        &mut arith_table_inputs,
                    );
                    // thread_controller.remove_working_thread();
                    // TODO! Implement prove drained_inputs (a chunk of operations)
                });
            }
        }
    }
}
