use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::Lookup0Trace;

pub struct Lookup0;

impl Lookup0 {
    const MY_NAME: &'static str = "Lookup_0";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup0
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = Lookup0Trace::new_zeroes();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let num_lookups = trace[0].sel.len();

        for j in 0..num_lookups {
            for i in 0..num_rows {
                // Assumes
                trace[i].f[2 * j] = rng.gen();
                trace[i].f[2 * j + 1] = rng.gen();
                let selected = rng.gen_bool(0.5);
                trace[i].sel[j] = F::from_bool(selected);

                // Proves
                trace[i].t[2 * j] = trace[i].f[2 * j];
                trace[i].t[2 * j + 1] = trace[i].f[2 * j + 1];
                if selected {
                    trace[i].mul[j] = F::one();
                }
            }
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
