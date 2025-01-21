use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;

use crate::SumBusTrace;

pub struct SumBus;

impl SumBus {
    const MY_NAME: &'static str = "SumBus  ";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField> WitnessComponent<F> for SumBus {
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut trace = SumBusTrace::new();
        let num_rows = trace.num_rows();

        log::debug!("{}: ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        for i in 0..num_rows {
            trace[i].a = F::from_canonical_usize(i);
            trace[i].b = trace[i].a;
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
