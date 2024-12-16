use std::sync::Arc;

use p3_field::PrimeField;

use proofman_common::{AirInstance, FromTrace, ProofCtx};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_pil::ArithRangeTableTrace;

use rayon::prelude::*;

use crate::ArithRangeTableSM;

pub struct ArithRangeTableInstance<F: PrimeField> {
    /// Arith range table state machine
    arith_range_table_sm: Arc<ArithRangeTableSM>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Arith range table trace
    trace: ArithRangeTableTrace<F>,
}

impl<F: PrimeField> ArithRangeTableInstance<F> {
    pub fn new(arith_range_table_sm: Arc<ArithRangeTableSM>, iectx: InstanceExpanderCtx) -> Self {
        Self { arith_range_table_sm, iectx, trace: ArithRangeTableTrace::<F>::new() }
    }
}

unsafe impl<F: PrimeField> Sync for ArithRangeTableInstance<F> {}

impl<F: PrimeField> Instance<F> for ArithRangeTableInstance<F> {
    fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        let mut multiplicity = self.arith_range_table_sm.detach_multiplicity();

        pctx.dctx_distribute_multiplicity(&mut multiplicity, self.iectx.global_idx);

        self.trace.buffer[0..ArithRangeTableTrace::<F>::NUM_ROWS]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, input)| input.multiplicity = F::from_canonical_u64(multiplicity[i]));

        Some(AirInstance::new_from_trace(FromTrace::new(&mut self.trace)))
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}
