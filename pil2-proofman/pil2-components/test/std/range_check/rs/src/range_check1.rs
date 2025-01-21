use std::sync::Arc;

use pil_std_lib::Std;
use witness::WitnessComponent;

use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::RangeCheck1Trace;

pub struct RangeCheck1<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> RangeCheck1<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "RngChck1";

    pub fn new(std_lib: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std_lib })
    }
}

impl<F: PrimeField> WitnessComponent<F> for RangeCheck1<F>
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = RangeCheck1Trace::new_zeroes();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let range1 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 8) - 1), Some(false));
        let range2 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 4) - 1), Some(false));
        let range3 = self.std_lib.get_range(BigInt::from(60), BigInt::from((1 << 16) - 1), Some(false));
        let range4 = self.std_lib.get_range(BigInt::from(8228), BigInt::from(17400), Some(false));

        for i in 0..num_rows {
            let selected1 = rng.gen_bool(0.5);
            trace[i].sel1 = F::from_bool(selected1);

            let selected2 = rng.gen_bool(0.5);
            trace[i].sel2 = F::from_bool(selected2);

            let selected3 = rng.gen_bool(0.5);
            trace[i].sel3 = F::from_bool(selected3);

            if selected1 {
                trace[i].a1 = F::from_canonical_u16(rng.gen_range(0..=(1 << 8) - 1));
                trace[i].a3 = F::from_canonical_u32(rng.gen_range(60..=(1 << 16) - 1));

                self.std_lib.range_check(trace[i].a1, F::one(), range1);
                self.std_lib.range_check(trace[i].a3, F::one(), range3);
            }

            if selected2 {
                trace[i].a2 = F::from_canonical_u8(rng.gen_range(0..=(1 << 4) - 1));
                trace[i].a4 = F::from_canonical_u16(rng.gen_range(8228..=17400));

                self.std_lib.range_check(trace[i].a2, F::one(), range2);
                self.std_lib.range_check(trace[i].a4, F::one(), range4);
            }

            if selected3 {
                trace[i].a5 = F::from_canonical_u16(rng.gen_range(0..=(1 << 8) - 1));

                self.std_lib.range_check(trace[i].a5, F::one(), range1);
            }
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
