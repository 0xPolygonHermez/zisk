use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::{AirInstance, FromTrace};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::ArithRangeTableTrace;
use ziskemu::EmuTrace;

use rayon::prelude::*;

use crate::ArithRangeTableSM;

pub struct ArithRangeTableInstance<F: PrimeField> {
    /// Witness manager
    wcm: Arc<WitnessManager<F>>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Arith range table state machine
    arith_range_table_sm: Arc<ArithRangeTableSM>,
}

impl<F: PrimeField> ArithRangeTableInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        arith_range_table_sm: Arc<ArithRangeTableSM>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        Self { wcm, iectx, arith_range_table_sm }
    }
}

unsafe impl<F: PrimeField> Sync for ArithRangeTableInstance<F> {}

impl<F: PrimeField> Instance for ArithRangeTableInstance<F> {
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
        let mut multiplicity = self.arith_range_table_sm.multiplicity.lock().unwrap();
        let mut multiplicity_ = std::mem::take(&mut *multiplicity);

        let ectx = self.wcm.get_ectx();
        let dctx = ectx.dctx.write().unwrap();

        let owner: usize = dctx.owner(self.iectx.instance_global_idx);
        dctx.distribute_multiplicity(&mut multiplicity_, owner);
        drop(dctx);

        let mut trace = ArithRangeTableTrace::<F>::new();

        trace.buffer[0..ArithRangeTableTrace::<F>::NUM_ROWS]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, input)| input.multiplicity = F::from_canonical_u64(multiplicity_[i]));

        let air_instance =
            AirInstance::new_from_trace(self.wcm.get_sctx(), FromTrace::new(&mut trace));

        self.wcm
            .get_pctx()
            .air_instance_repo
            .add_air_instance(air_instance, Some(self.iectx.instance_global_idx));

        Ok(())
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}
