use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::{ARITH_AIR_IDS, ARITH_RANGE_TABLE_AIR_IDS, ARITH_TABLE_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{arith_full, ArithFullSM, ArithRangeTableSM, ArithTableSM};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct ArithSM<F> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,

    arith_full_sm: Arc<ArithFullSM<F>>,
    arith_table_sm: Arc<ArithTableSM<F>>,
    arith_range_table_sm: Arc<ArithRangeTableSM<F>>,
}

impl<F: Field> ArithSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let arith_table_sm = ArithTableSM::new(wcm.clone(), ZISK_AIRGROUP_ID, ARITH_TABLE_AIR_IDS);
        let arith_range_table_sm =
            ArithRangeTableSM::new(wcm.clone(), ZISK_AIRGROUP_ID, ARITH_RANGE_TABLE_AIR_IDS);
        let arith_full_sm = ArithFullSM::new(
            wcm.clone(),
            arith_table_sm.clone(),
            arith_range_table_sm.clone(),
            ZISK_AIRGROUP_ID,
            ARITH_AIR_IDS,
        );
        let arith_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            // threads_controller: Arc::new(ThreadController::new()),
            inputs: Mutex::new(Vec::new()),
            arith_full_sm,
            arith_table_sm,
            arith_range_table_sm,
        };
        let arith_sm = Arc::new(arith_sm);

        wcm.register_component(arith_sm.clone(), None, None);

        arith_sm.arith_full_sm.register_predecessor();

        arith_sm
    }
    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.arith_full_sm.unregister_predecessor();
        }
    }
    pub fn prove_instance(
        &self,
        operations: Vec<ZiskRequiredOperation>,
        prover_buffer: &mut [F],
        offset: u64,
    ) {
        self.arith_full_sm.prove_instance(operations, prover_buffer, offset);
    }
}

impl<F: Field> WitnessComponent<F> for ArithSM<F> {}

impl<F: Field> Provable<ZiskRequiredOperation, OpResult> for ArithSM<F> {
    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        while operations.len() >= PROVE_CHUNK_SIZE || (drain && !operations.is_empty()) {
            if drain && !operations.is_empty() {
                // println!("ArithSM: Draining inputs");
            }

            let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, operations.len());
            let drained_inputs = operations[..num_drained].to_vec();
            let arith_full_sm_cloned = self.arith_full_sm.clone();

            // self.threads_controller.add_working_thread();
            // let thread_controller = self.threads_controller.clone();

            arith_full_sm_cloned.prove(&drained_inputs, drain, scope);
        }
    }
}
