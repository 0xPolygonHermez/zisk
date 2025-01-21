use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::BothBusesTrace;

pub struct BothBuses;

impl BothBuses {
    const MY_NAME: &'static str = "BothBus ";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField> WitnessComponent<F> for BothBuses
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = BothBusesTrace::new();
        let num_rows = trace.num_rows();

        log::debug!("{}: ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        for i in 0..num_rows {
            trace[i].a = F::from_canonical_u64(rng.gen_range(0..=(1 << 63) - 1));
            trace[i].b = trace[i].a;

            trace[i].c = F::from_canonical_usize(i);
            trace[i].d = trace[i].c;
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
