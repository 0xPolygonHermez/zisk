use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::{AirInstance, FromTrace};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::BinaryExtensionTableTrace;
use ziskemu::EmuTrace;

use rayon::prelude::*;

use crate::BinaryExtensionTableSM;

pub struct BinaryExtensionTableInstance<F: PrimeField> {
    /// Witness manager
    wcm: Arc<WitnessManager<F>>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Binary extension table state machine
    binary_extension_table_sm: Arc<BinaryExtensionTableSM>,
}

impl<F: PrimeField> BinaryExtensionTableInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        binary_extension_table_sm: Arc<BinaryExtensionTableSM>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        Self { wcm, iectx, binary_extension_table_sm }
    }
}

unsafe impl<F: PrimeField> Sync for BinaryExtensionTableInstance<F> {}

impl<F: PrimeField> Instance<F> for BinaryExtensionTableInstance<F> {
    fn collect(
        &mut self,
        _: &ZiskRom,
        _: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        let mut multiplicity = self.binary_extension_table_sm.multiplicity.lock().unwrap();
        let mut multiplicity = std::mem::take(&mut *multiplicity);

        self.wcm.get_ectx().dctx_distribute_multiplicity(&mut multiplicity, self.iectx.global_idx);

        let mut trace = BinaryExtensionTableTrace::<F>::new();
        trace.buffer[0..BinaryExtensionTableTrace::<F>::NUM_ROWS]
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
