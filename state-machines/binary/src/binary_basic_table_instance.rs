use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::AirInstance;
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::BinaryTableTrace;
use ziskemu::EmuTrace;

use rayon::prelude::*;

use crate::BinaryBasicTableSM;

pub struct BinaryBasicTableInstance<F: PrimeField> {
    /// Witness manager
    wcm: Arc<WitnessManager<F>>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Binary basic table state machine
    binary_basic_table_sm: Arc<BinaryBasicTableSM<F>>,
}

impl<F: PrimeField> BinaryBasicTableInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        binary_basic_table_sm: Arc<BinaryBasicTableSM<F>>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        Self { wcm, iectx, binary_basic_table_sm }
    }
}

unsafe impl<F: PrimeField> Sync for BinaryBasicTableInstance<F> {}

impl<F: PrimeField> Instance for BinaryBasicTableInstance<F> {
    fn expand(
        &mut self,
        _: &ZiskRom,
        _: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn prove(
        &mut self,
        _min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let ectx = self.wcm.get_ectx();
        let dctx = ectx.dctx.write().unwrap();

        let owner: usize = dctx.owner(self.iectx.instance_global_idx);

        let mut multiplicity = self.binary_basic_table_sm.multiplicity.lock().unwrap();
        let mut multiplicity_ = std::mem::take(&mut *multiplicity);

        dctx.distribute_multiplicity(&mut multiplicity_, owner);
        drop(dctx);

        // if is_mine {
        let pctx = self.wcm.get_pctx();
        let air = pctx.pilout.get_air(self.iectx.plan.airgroup_id, self.iectx.plan.air_id);
        let binary_basic_trace = BinaryTableTrace::<F>::new(air.num_rows());

        let buffer = binary_basic_trace.buffer;
        let mut buffer: Vec<F> = unsafe { std::mem::transmute(buffer) };

        buffer[0..air.num_rows()]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, input)| *input = F::from_canonical_u64(multiplicity_[i]));

        let air_instance = AirInstance::new(
            self.wcm.get_sctx(),
            self.iectx.plan.airgroup_id,
            self.iectx.plan.air_id,
            None,
            buffer,
        );

        let air_instance_repo = &self.wcm.get_pctx().air_instance_repo;
        air_instance_repo.add_air_instance(air_instance, Some(self.iectx.instance_global_idx));
        // }

        Ok(())
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}
