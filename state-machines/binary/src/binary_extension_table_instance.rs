use std::sync::Arc;

use p3_field::PrimeField;

use proofman_common::{AirInstance, FromTrace, ProofCtx};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_pil::BinaryExtensionTableTrace;

use rayon::prelude::*;

use crate::BinaryExtensionTableSM;

pub struct BinaryExtensionTableInstance<F: PrimeField> {
    /// Binary extension table state machine
    binary_extension_table_sm: Arc<BinaryExtensionTableSM>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Binary extension table trace
    trace: BinaryExtensionTableTrace<F>,
}

impl<F: PrimeField> BinaryExtensionTableInstance<F> {
    pub fn new(
        binary_extension_table_sm: Arc<BinaryExtensionTableSM>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        Self { binary_extension_table_sm, iectx, trace: BinaryExtensionTableTrace::<F>::new() }
    }
}

unsafe impl<F: PrimeField> Sync for BinaryExtensionTableInstance<F> {}

impl<F: PrimeField> Instance<F> for BinaryExtensionTableInstance<F> {
    fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        let mut multiplicity = self.binary_extension_table_sm.detach_multiplicity();

        pctx.dctx_distribute_multiplicity(&mut multiplicity, self.iectx.global_idx);

        self.trace.buffer[0..BinaryExtensionTableTrace::<F>::NUM_ROWS]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, input)| input.multiplicity = F::from_canonical_u64(multiplicity[i]));

        let instance = AirInstance::new_from_trace(FromTrace::new(&mut self.trace));

        Some(instance)
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}
