use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::Lookup2_12Trace;

pub struct Lookup2_12;

impl Lookup2_12 {
    const MY_NAME: &'static str = "Lkup2_12";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup2_12
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = Lookup2_12Trace::new();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        // TODO: Add the ability to send inputs to lookup3
        //       and consequently add random selectors

        for i in 0..num_rows {
            // Inner lookups
            trace[i].a1 = rng.gen();
            trace[i].b1 = rng.gen();
            trace[i].c1 = trace[i].a1;
            trace[i].d1 = trace[i].b1;

            trace[i].a3 = rng.gen();
            trace[i].b3 = rng.gen();
            trace[i].c2 = trace[i].a3;
            trace[i].d2 = trace[i].b3;
            let selected = rng.gen_bool(0.5);
            trace[i].sel1 = F::from_bool(selected);
            if selected {
                trace[i].mul = trace[i].sel1;
            }

            // Outer lookups
            trace[i].a2 = F::from_canonical_usize(i);
            trace[i].b2 = F::from_canonical_usize(i);

            trace[i].a4 = F::from_canonical_usize(i);
            trace[i].b4 = F::from_canonical_usize(i);
            trace[i].sel2 = F::from_bool(true);
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
