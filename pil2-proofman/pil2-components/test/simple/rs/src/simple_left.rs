use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField64;
use rand::{distributions::Standard, prelude::Distribution, seq::SliceRandom, Rng};

use crate::SimpleLeftTrace;

pub struct SimpleLeft;

impl SimpleLeft {
    const MY_NAME: &'static str = "SimLeft ";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField64 + Copy> WitnessComponent<F> for SimpleLeft
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = SimpleLeftTrace::new();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        // Assumes
        for i in 0..num_rows {
            trace[i].a = F::from_canonical_u64(rng.gen_range(0..=(1 << 63) - 1));
            trace[i].b = F::from_canonical_u64(rng.gen_range(0..=(1 << 63) - 1));

            trace[i].e = F::from_canonical_u8(200);
            trace[i].f = F::from_canonical_u8(201);

            trace[i].g = F::from_canonical_usize(i);
            trace[i].h = F::from_canonical_usize(num_rows - i - 1);
        }

        let mut indices: Vec<usize> = (0..num_rows).collect();
        indices.shuffle(&mut rng);

        // Proves
        for i in 0..num_rows {
            // We take a random permutation of the indices to show that the permutation check is passing
            trace[i].c = trace[indices[i]].a;
            trace[i].d = trace[indices[i]].b;
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
