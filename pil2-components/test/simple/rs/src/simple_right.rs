use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField64;
use rand::{distributions::Standard, prelude::Distribution};

use crate::SimpleRightTrace;

pub struct SimpleRight;

impl SimpleRight {
    const MY_NAME: &'static str = "SimRight";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField64 + Copy> WitnessComponent<F> for SimpleRight
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut trace = SimpleRightTrace::new();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        // Proves
        for i in 0..num_rows {
            trace[i].a = F::from_canonical_u8(200);
            trace[i].b = F::from_canonical_u8(201);

            trace[i].c = F::from_canonical_usize(i);
            trace[i].d = F::from_canonical_usize(num_rows - i - 1);

            trace[i].mul = F::from_canonical_usize(1);
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
