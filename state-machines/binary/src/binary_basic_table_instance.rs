use std::sync::Arc;

use p3_field::PrimeField;

use proofman::WitnessManager;
use proofman_common::{AirInstance, FromTrace};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_pil::BinaryTableTrace;

use rayon::prelude::*;

use crate::BinaryBasicTableSM;

pub struct BinaryBasicTableInstance<F: PrimeField> {
    /// Witness manager
    wcm: Arc<WitnessManager<F>>,

    /// Binary basic table state machine
    binary_basic_table_sm: Arc<BinaryBasicTableSM>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Binary basic table trace
    trace: BinaryTableTrace<F>,
}

impl<F: PrimeField> BinaryBasicTableInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        binary_basic_table_sm: Arc<BinaryBasicTableSM>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        Self { wcm, binary_basic_table_sm, iectx, trace: BinaryTableTrace::<F>::new() }
    }
}

unsafe impl<F: PrimeField> Sync for BinaryBasicTableInstance<F> {}

impl<F: PrimeField> Instance<F> for BinaryBasicTableInstance<F> {
    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        let mut multiplicity = self.binary_basic_table_sm.detach_multiplicity();

        self.wcm.get_ectx().dctx_distribute_multiplicity(&mut multiplicity, self.iectx.global_idx);

        self.trace.buffer[0..BinaryTableTrace::<F>::NUM_ROWS]
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
