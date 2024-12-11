use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use p3_field::PrimeField;
use proofman::{WitnessComponent, WitnessManager};
use sm_binary::BinarySM;
use zisk_core::ZiskRequiredOperation;
use zisk_pil::{ARITH_AIR_IDS, ARITH_RANGE_TABLE_AIR_IDS, ARITH_TABLE_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{ArithFullSM, ArithRangeTableSM, ArithTableSM};

#[allow(dead_code)]
pub struct ArithSM<F: PrimeField> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    arith_full_sm: Arc<ArithFullSM<F>>,
    arith_table_sm: Arc<ArithTableSM<F>>,
    arith_range_table_sm: Arc<ArithRangeTableSM<F>>,
}

impl<F: PrimeField> ArithSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, binary_sm: Arc<BinarySM<F>>) -> Arc<Self> {
        let arith_table_sm = ArithTableSM::new(wcm.clone(), ZISK_AIRGROUP_ID, ARITH_TABLE_AIR_IDS);
        let arith_range_table_sm =
            ArithRangeTableSM::new(wcm.clone(), ZISK_AIRGROUP_ID, ARITH_RANGE_TABLE_AIR_IDS);
        let arith_full_sm = ArithFullSM::new(
            wcm.clone(),
            arith_table_sm.clone(),
            arith_range_table_sm.clone(),
            binary_sm,
            ZISK_AIRGROUP_ID,
            ARITH_AIR_IDS,
        );
        let arith_sm = Self {
            registered_predecessors: AtomicU32::new(0),
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
    pub fn prove_instance(&self, operations: Vec<ZiskRequiredOperation>, prover_buffer: &mut [F]) {
        self.arith_full_sm.prove_instance(operations, prover_buffer);
    }
}

impl<F: PrimeField> WitnessComponent<F> for ArithSM<F> {}
