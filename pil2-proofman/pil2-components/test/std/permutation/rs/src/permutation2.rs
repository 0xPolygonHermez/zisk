use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;

use crate::Permutation2_6Trace;

pub struct Permutation2;

impl Permutation2 {
    const MY_NAME: &'static str = "Perm2   ";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Permutation2 {
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut trace = Permutation2_6Trace::new();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        // Note: Here it is assumed that num_rows of permutation2 is equal to
        //       the sum of num_rows of each variant of permutation1.
        //       Ohterwise, the permutation check cannot be satisfied.
        // Proves
        for i in 0..num_rows {
            trace[i].c1 = F::from_canonical_u8(200);
            trace[i].d1 = F::from_canonical_u8(201);

            trace[i].c2 = F::from_canonical_u8(100);
            trace[i].d2 = F::from_canonical_u8(101);

            trace[i].sel = F::from_bool(true);
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
