use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::Permutation1_8Trace;

pub struct Permutation1_8;

impl Permutation1_8 {
    const MY_NAME: &'static str = "Perm1_8 ";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Permutation1_8
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();
        let mut trace = Permutation1_8Trace::new();
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

            trace[i].sel1 = F::one();
            trace[i].sel3 = F::one(); // F::from_canonical_u8(rng.gen_range(0..=1));
        }

        // TODO: Add the permutation of indexes

        // Proves
        for i in 0..num_rows {
            let index = num_rows - i - 1;
            // let mut index = rng.gen_range(0..num_rows);
            trace[i].c1 = trace[index].a1;
            trace[i].d1 = trace[index].b1;

            // index = rng.gen_range(0..num_rows);
            trace[i].c2 = trace[index].a3;
            trace[i].d2 = trace[index].b3;

            trace[i].sel2 = trace[i].sel1;
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
