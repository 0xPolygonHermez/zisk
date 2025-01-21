use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, seq::SliceRandom, Rng};

use crate::Permutation1_6Trace;

pub struct Permutation1_6;

impl Permutation1_6 {
    const MY_NAME: &'static str = "Perm1_6 ";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}
impl<F: PrimeField + Copy> WitnessComponent<F> for Permutation1_6
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = Permutation1_6Trace::new();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        // TODO: Add the ability to send inputs to permutation2
        //       and consequently add random selectors

        // Assumes
        for i in 0..num_rows {
            trace[i].a1 = rng.gen();
            trace[i].b1 = rng.gen();

            trace[i].a2 = F::from_canonical_u8(200);
            trace[i].b2 = F::from_canonical_u8(201);

            trace[i].a3 = rng.gen();
            trace[i].b3 = rng.gen();

            trace[i].a4 = F::from_canonical_u8(100);
            trace[i].b4 = F::from_canonical_u8(101);

            trace[i].sel1 = F::from_bool(rng.gen_bool(0.5));
            trace[i].sel3 = F::one();
        }

        let mut indices: Vec<usize> = (0..num_rows).collect();
        indices.shuffle(&mut rng);

        // Proves
        for i in 0..num_rows {
            // We take a random permutation of the indices to show that the permutation check is passing
            trace[i].c1 = trace[indices[i]].a1;
            trace[i].d1 = trace[indices[i]].b1;

            trace[i].c2 = trace[indices[i]].a3;
            trace[i].d2 = trace[indices[i]].b3;

            trace[i].sel2 = trace[indices[i]].sel1;
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());

        let mut trace2 = Permutation1_6Trace::new();

        // Assumes
        for i in 0..num_rows {
            trace2[i].a1 = rng.gen();
            trace2[i].b1 = rng.gen();

            trace2[i].a2 = F::from_canonical_u8(200);
            trace2[i].b2 = F::from_canonical_u8(201);

            trace2[i].a3 = rng.gen();
            trace2[i].b3 = rng.gen();

            trace2[i].a4 = F::from_canonical_u8(100);
            trace2[i].b4 = F::from_canonical_u8(101);

            trace2[i].sel1 = F::from_bool(rng.gen_bool(0.5));
            trace2[i].sel3 = F::one();
        }

        let mut indices: Vec<usize> = (0..num_rows).collect();
        indices.shuffle(&mut rng);

        // Proves
        for i in 0..num_rows {
            // We take a random permutation of the indices to show that the permutation check is passing
            trace2[i].c1 = trace2[indices[i]].a1;
            trace2[i].d1 = trace2[indices[i]].b1;

            trace2[i].c2 = trace2[indices[i]].a3;
            trace2[i].d2 = trace2[indices[i]].b3;

            trace2[i].sel2 = trace2[indices[i]].sel1;
        }

        let air_instance2 = AirInstance::new_from_trace(FromTrace::new(&mut trace2));
        add_air_instance::<F>(air_instance2, pctx.clone());
    }
}
