use std::sync::Arc;

use p3_field::PrimeField;

use proofman_common::{AirInstance, FromTrace, ProofCtx};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_pil::ArithTableTrace;

use rayon::prelude::*;

use crate::ArithTableSM;

pub struct ArithTableInstance<F: PrimeField> {
    /// Arith table state machine
    arith_table_sm: Arc<ArithTableSM>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Arith table trace
    trace: ArithTableTrace<F>,
}

impl<F: PrimeField> ArithTableInstance<F> {
    pub fn new(arith_table_sm: Arc<ArithTableSM>, iectx: InstanceExpanderCtx) -> Self {
        Self { arith_table_sm, iectx, trace: ArithTableTrace::<F>::new() }
    }
}

unsafe impl<F: PrimeField> Sync for ArithTableInstance<F> {}

impl<F: PrimeField> Instance<F> for ArithTableInstance<F> {
    fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        let mut multiplicity = self.arith_table_sm.detach_multiplicity();

        pctx.dctx_distribute_multiplicity(&mut multiplicity, self.iectx.global_idx);

        self.trace.buffer[0..ArithTableTrace::<F>::NUM_ROWS]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, input)| input.multiplicity = F::from_canonical_u64(multiplicity[i]));

        Some(AirInstance::new_from_trace(FromTrace::new(&mut self.trace)))
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}
