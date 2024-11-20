use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::ArithTableInputs;
use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use rayon::prelude::*;
use sm_common::create_prover_buffer;
use zisk_pil::{ARITH_TABLE_AIR_IDS, ZISK_AIRGROUP_ID};

pub struct ArithTableSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    num_rows: usize,
    multiplicity: Mutex<Vec<u64>>,
}

impl<F: Field> ArithTableSM<F> {
    const MY_NAME: &'static str = "ArithT  ";

    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, ARITH_TABLE_AIR_IDS[0]);
        let _arith_table_sm = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            num_rows: air.num_rows(),
            multiplicity: Mutex::new(vec![0; air.num_rows()]),
        };
        let arith_table_sm = Arc::new(_arith_table_sm);

        wcm.register_component(arith_table_sm.clone(), Some(airgroup_id), Some(air_ids));

        arith_table_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.create_air_instance();
        }
    }
    pub fn process_slice(&self, inputs: &ArithTableInputs) {
        // Create the trace vector
        let mut _multiplicity = self.multiplicity.lock().unwrap();

        info!("{}: ··· process multiplicity", Self::MY_NAME);
        for (row, value) in inputs {
            info!("{}: ··· Processing row {} with value {}", Self::MY_NAME, row, value);
            _multiplicity[row] += value;
        }
    }
    pub fn create_air_instance(&self) {
        let ectx = self.wcm.get_ectx();
        let mut dctx: std::sync::RwLockWriteGuard<'_, proofman_common::DistributionCtx> =
            ectx.dctx.write().unwrap();
        let mut multiplicity = self.multiplicity.lock().unwrap();

        let (is_myne, instance_global_idx) =
            dctx.add_instance(ZISK_AIRGROUP_ID, ARITH_TABLE_AIR_IDS[0], 1);
        let owner: usize = dctx.owner(instance_global_idx);

        let mut multiplicity_ = std::mem::take(&mut *multiplicity);
        dctx.distribute_multiplicity(&mut multiplicity_, owner);

        if is_myne {
            // Create the prover buffer
            let (mut prover_buffer, offset) = create_prover_buffer(
                &self.wcm.get_ectx(),
                &self.wcm.get_sctx(),
                ZISK_AIRGROUP_ID,
                ARITH_TABLE_AIR_IDS[0],
            );
            prover_buffer[offset as usize..offset as usize + self.num_rows]
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, input)| *input = F::from_canonical_u64(multiplicity_[i]));

            info!(
                "{}: ··· Creating Binary basic table instance [{} rows filled 100%]",
                Self::MY_NAME,
                self.num_rows,
            );
            let air_instance = AirInstance::new(
                self.wcm.get_sctx(),
                ZISK_AIRGROUP_ID,
                ARITH_TABLE_AIR_IDS[0],
                None,
                prover_buffer,
            );
            self.wcm
                .get_pctx()
                .air_instance_repo
                .add_air_instance(air_instance, Some(instance_global_idx));
        }
    }
}

impl<F: Field> WitnessComponent<F> for ArithTableSM<F> {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx<F>>,
        _sctx: Arc<SetupCtx<F>>,
    ) {
    }
}
