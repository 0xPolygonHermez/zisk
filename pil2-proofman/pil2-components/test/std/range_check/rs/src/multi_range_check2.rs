use std::sync::Arc;

use pil_std_lib::Std;
use witness::WitnessComponent;

use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::MultiRangeCheck2Trace;

pub struct MultiRangeCheck2<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> MultiRangeCheck2<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "MtRngCh2";

    pub fn new(std_lib: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std_lib })
    }
}

impl<F: PrimeField> WitnessComponent<F> for MultiRangeCheck2<F>
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = MultiRangeCheck2Trace::new_zeroes();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let range1 = self.std_lib.get_range(BigInt::from(1 << 5), BigInt::from((1 << 8) - 1), Some(false));
        let range2 = self.std_lib.get_range(BigInt::from(1 << 8), BigInt::from((1 << 9) - 1), Some(false));
        let range3 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 7) - 1), Some(false));
        let range4 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 4) - 1), Some(false));

        for i in 0..num_rows {
            let selected1 = rng.gen_bool(0.5);
            let range_selector1 = rng.gen_bool(0.5);
            trace[i].sel[0] = F::from_bool(selected1);
            trace[i].range_sel[0] = F::from_bool(range_selector1);

            let selected2 = rng.gen_bool(0.5);
            let range_selector2 = rng.gen_bool(0.5);
            trace[i].sel[1] = F::from_bool(selected2);
            trace[i].range_sel[1] = F::from_bool(range_selector2);

            if selected1 {
                if range_selector1 {
                    trace[i].a[0] = F::from_canonical_u16(rng.gen_range((1 << 5)..=(1 << 8) - 1));

                    self.std_lib.range_check(trace[i].a[0], F::one(), range1);
                } else {
                    trace[i].a[0] = F::from_canonical_u16(rng.gen_range((1 << 8)..=(1 << 9) - 1));

                    self.std_lib.range_check(trace[i].a[0], F::one(), range2);
                }
            }

            if selected2 {
                if range_selector2 {
                    trace[i].a[1] = F::from_canonical_u16(rng.gen_range(0..=(1 << 7) - 1));

                    self.std_lib.range_check(trace[i].a[1], F::one(), range3);
                } else {
                    trace[i].a[1] = F::from_canonical_u16(rng.gen_range(0..=(1 << 4) - 1));

                    self.std_lib.range_check(trace[i].a[1], F::one(), range4);
                }
            }
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
