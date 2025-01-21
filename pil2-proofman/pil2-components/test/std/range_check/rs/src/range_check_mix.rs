use std::sync::Arc;

use pil_std_lib::Std;
use witness::WitnessComponent;

use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::RangeCheckMixTrace;

pub struct RangeCheckMix<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> RangeCheckMix<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "RngChMix";

    pub fn new(std_lib: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std_lib })
    }
}

impl<F: PrimeField> WitnessComponent<F> for RangeCheckMix<F>
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();

        let mut trace = RangeCheckMixTrace::new_zeroes();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let range1 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 8) - 1), None);
        let range2 = self.std_lib.get_range(BigInt::from(50), BigInt::from((1 << 7) - 1), None);
        let range3 = self.std_lib.get_range(BigInt::from(-1), BigInt::from(1 << 3), None);
        let range4 = self.std_lib.get_range(BigInt::from(-(1 << 7) + 1), BigInt::from(-50), None);

        let range5 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 7) - 1), Some(false));
        let range6 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 4) - 1), Some(false));
        let range7 = self.std_lib.get_range(BigInt::from(1 << 5), BigInt::from((1 << 8) - 1), Some(false));
        let range8 = self.std_lib.get_range(BigInt::from(1 << 8), BigInt::from((1 << 9) - 1), Some(false));

        let range9 = self.std_lib.get_range(BigInt::from(5225), BigInt::from(29023), Some(false));
        // let range10 = self.std_lib.get_range(BigInt::from(-8719), BigInt::from(-7269), Some(false));
        let range11 = self.std_lib.get_range(BigInt::from(-10), BigInt::from(10), Some(false));

        for i in 0..num_rows {
            // First interface
            trace[i].a[0] = F::from_canonical_u16(rng.gen_range(0..=(1 << 8) - 1));
            self.std_lib.range_check(trace[i].a[0], F::one(), range1);

            trace[i].a[1] = F::from_canonical_u8(rng.gen_range(50..=(1 << 7) - 1));
            self.std_lib.range_check(trace[i].a[1], F::one(), range2);

            let mut a2_val: i128 = rng.gen_range(-1..=2i128.pow(3));
            if a2_val < 0 {
                a2_val += F::order().to_i128().unwrap();
            }
            trace[i].a[2] = F::from_canonical_u64(a2_val as u64);
            self.std_lib.range_check(trace[i].a[2], F::one(), range3);

            let a3_val = rng.gen_range(-(2i128.pow(7)) + 1..=-50) + F::order().to_i128().unwrap();
            trace[i].a[3] = F::from_canonical_u64(a3_val as u64);
            self.std_lib.range_check(trace[i].a[3], F::one(), range4);

            // Second interface
            let range_selector1 = rng.gen_bool(0.5);
            trace[i].range_sel[0] = F::from_bool(range_selector1);

            let range_selector2 = rng.gen_bool(0.5);
            trace[i].range_sel[1] = F::from_bool(range_selector2);

            if range_selector1 {
                trace[i].b[0] = F::from_canonical_u16(rng.gen_range(0..=(1 << 7) - 1));

                self.std_lib.range_check(trace[i].b[0], F::one(), range5);
            } else {
                trace[i].b[0] = F::from_canonical_u16(rng.gen_range(0..=(1 << 4) - 1));

                self.std_lib.range_check(trace[i].b[0], F::one(), range6);
            }

            if range_selector2 {
                trace[i].b[1] = F::from_canonical_u16(rng.gen_range((1 << 5)..=(1 << 8) - 1));

                self.std_lib.range_check(trace[i].b[1], F::one(), range7);
            } else {
                trace[i].b[1] = F::from_canonical_u16(rng.gen_range((1 << 8)..=(1 << 9) - 1));

                self.std_lib.range_check(trace[i].b[1], F::one(), range8);
            }

            // Third interface
            let range = rng.gen_range(0..=2);

            match range {
                0 => {
                    trace[i].range_sel[2] = F::one();
                    trace[i].c[0] = F::from_canonical_u32(rng.gen_range(5225..=29023));

                    self.std_lib.range_check(trace[i].c[0], F::one(), range9);
                }
                1 => {
                    trace[i].range_sel[3] = F::one();
                    let mut colu_val: i128 = rng.gen_range(-10..=10);
                    if colu_val < 0 {
                        colu_val += F::order().to_i128().unwrap();
                    }
                    trace[i].c[0] = F::from_canonical_u64(colu_val as u64);

                    self.std_lib.range_check(trace[i].c[0], F::one(), range11);
                }
                2 => {
                    trace[i].range_sel[4] = F::one();
                    trace[i].c[0] = F::from_canonical_u32(rng.gen_range(0..=(1 << 7) - 1));

                    self.std_lib.range_check(trace[i].c[0], F::one(), range5);
                }
                _ => panic!("Invalid range"),
            }
        }

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
