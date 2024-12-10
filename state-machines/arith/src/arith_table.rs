use std::sync::{Arc, Mutex};

use crate::ArithTableInputs;
use log::info;
use p3_field::Field;
use proofman::WitnessManager;
use proofman_common::{AirInstance, FromTrace};

use rayon::prelude::*;
use zisk_pil::{ArithTableTrace, ARITH_TABLE_AIR_IDS, ZISK_AIRGROUP_ID};

pub struct ArithTableSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Inputs
    pub multiplicity: Mutex<Vec<u64>>,
}

impl<F: Field> ArithTableSM<F> {
    const MY_NAME: &'static str = "ArithT  ";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        Arc::new(Self {
            wcm: wcm.clone(),
            multiplicity: Mutex::new(vec![0; ArithTableTrace::<F>::NUM_ROWS]),
        })
    }

    pub fn process_slice(&self, inputs: &ArithTableInputs) {
        // Create the trace vector
        let mut multiplicity = self.multiplicity.lock().unwrap();

        info!("{}: ··· Processing multiplicity", Self::MY_NAME);
        for (row, value) in inputs {
            info!("{}: ··· Processing row {} with value {}", Self::MY_NAME, row, value);
            multiplicity[row] += value;
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
            let mut trace = ArithTableTrace::new();

            trace.buffer[0..ArithTableTrace::<F>::NUM_ROWS].par_iter_mut().enumerate().for_each(
                |(i, input)| input.multiplicity = F::from_canonical_u64(multiplicity_[i]),
            );

            info!(
                "{}: ··· Creating arith table instance [{} rows filled 100%]",
                Self::MY_NAME,
                ArithTableTrace::<F>::NUM_ROWS,
            );

            let air_instance =
                AirInstance::new_from_trace(self.wcm.get_sctx(), FromTrace::new(&mut trace));

            self.wcm
                .get_pctx()
                .air_instance_repo
                .add_air_instance(air_instance, Some(instance_global_idx));
        }
    }
}
