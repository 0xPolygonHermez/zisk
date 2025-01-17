use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{DirectUpdateProdLocalTrace, DirectUpdateProdLocalAirValues, DirectUpdatePublicValues, DirectUpdateProofValues};

pub struct DirectUpdateProdLocal;

impl DirectUpdateProdLocal {
    const MY_NAME: &'static str = "DUPL    ";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for DirectUpdateProdLocal
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = DirectUpdateProdLocalTrace::new_zeroes();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let chosen_index = rng.gen_range(0..=num_rows - 1);
        let mut values: [F; 6] = [F::zero(); 6];
        for i in 0..num_rows {
            for j in 0..2 {
                trace[i].a[j] = F::from_canonical_u64(rng.gen_range(0..=(1 << 63) - 1));
                trace[i].b[j] = F::from_canonical_u64(rng.gen_range(0..=(1 << 63) - 1));
                trace[i].c[j] = F::from_canonical_u64(rng.gen_range(0..=(1 << 63) - 1));
            }

            if i == chosen_index {
                trace[i].perform_operation = F::from_bool(true);
                values[0] = trace[i].a[0];
                values[1] = trace[i].a[1];
                values[2] = trace[i].b[0];
                values[3] = trace[i].b[1];
                values[4] = trace[i].c[0];
                values[5] = trace[i].c[1];
            }
        }

        // Set public values
        let mut public_values = DirectUpdatePublicValues::from_vec_guard(pctx.get_publics());
        public_values.a_public[0] = values[0];
        public_values.a_public[1] = values[1];

        // Set proof values
        let mut proof_values = DirectUpdateProofValues::from_vec_guard(pctx.get_proof_values());
        proof_values.b_proofval_0 = values[2];
        proof_values.b_proofval_1 = values[3];

        // Choose one direct update
        let mut air_values = DirectUpdateProdLocalAirValues::<F>::new();
        air_values.c_airval[0] = values[4];
        air_values.c_airval[1] = values[5];

        // Choose one direct update
        let h = rng.gen_bool(0.5);
        air_values.perform_direct_update[0] = F::from_bool(h);
        air_values.perform_direct_update[1] = F::from_bool(!h);

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
