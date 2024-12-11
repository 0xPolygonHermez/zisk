use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::{AirInstance, FromTrace};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::ArithTableTrace;
use ziskemu::EmuTrace;

use rayon::prelude::*;

use crate::ArithTableSM;

pub struct ArithTableInstance<F: PrimeField> {
    /// Witness manager
    wcm: Arc<WitnessManager<F>>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Arith table state machine
    arith_table_sm: Arc<ArithTableSM>,
}

impl<F: PrimeField> ArithTableInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        arith_table_sm: Arc<ArithTableSM>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        Self { wcm, iectx, arith_table_sm }
    }
}

unsafe impl<F: PrimeField> Sync for ArithTableInstance<F> {}

impl<F: PrimeField> Instance<F> for ArithTableInstance<F> {
    fn collect(
        &mut self,
        _: &ZiskRom,
        _: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        let mut multiplicity = self.arith_table_sm.multiplicity.lock().unwrap();
        let mut multiplicity = std::mem::take(&mut *multiplicity);

        self.wcm.get_ectx().dctx_distribute_multiplicity(&mut multiplicity, self.iectx.global_idx);

        let mut trace = ArithTableTrace::<F>::new();
        trace.buffer[0..ArithTableTrace::<F>::NUM_ROWS]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, input)| input.multiplicity = F::from_canonical_u64(multiplicity[i]));

        let instance = AirInstance::new_from_trace(self.wcm.get_sctx(), FromTrace::new(&mut trace));

        Some(instance)
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}
