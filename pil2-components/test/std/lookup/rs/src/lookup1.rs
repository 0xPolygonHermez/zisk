use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::Lookup1Trace;

pub struct Lookup1;

impl Lookup1 {
    const MY_NAME: &'static str = "Lookup_1";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup1
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = Lookup1Trace::new();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let num_lookups = trace[0].sel.len();

        for i in 0..num_rows {
            let val = rng.gen();
            let mut n_sel = 0;
            for j in 0..num_lookups {
                trace[i].f[j] = val;
                let selected = rng.gen_bool(0.5);
                trace[i].sel[j] = F::from_bool(selected);
                if selected {
                    n_sel += 1;
                }
            }
            trace[i].t = val;
            trace[i].mul = F::from_canonical_usize(n_sel);
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
