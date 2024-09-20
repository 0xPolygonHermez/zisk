use core::panic;
use std::sync::Arc;

use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{RangeCheck40Trace, RANGE_CHECK_4_AIRGROUP_ID, RANGE_CHECK_4_AIR_IDS};

pub struct RangeCheck4<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> RangeCheck4<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "RangeCheck4";

    pub fn new(wcm: Arc<WitnessManager<F>>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let range_check4 = Arc::new(Self { std_lib });

        wcm.register_component(
            range_check4.clone(),
            Some(RANGE_CHECK_4_AIRGROUP_ID),
            Some(RANGE_CHECK_4_AIR_IDS),
        );

        // Register dependency relations
        range_check4.std_lib.register_predecessor();

        range_check4
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of the air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("RangeCheck4".into(), RANGE_CHECK_4_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(
            RANGE_CHECK_4_AIRGROUP_ID,
            RANGE_CHECK_4_AIR_IDS[0],
            None,
            buffer,
        );
        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField> WitnessComponent<F> for RangeCheck4<F>
where
    Standard: Distribution<F>,
{
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>
    ) {
        let mut rng = rand::thread_rng();

        log::info!(
            "{}: Initiating witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            "RangeCheck4",
            stage
        );

        if stage == 1 {
            let (buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("RangeCheck4".into(), RANGE_CHECK_4_AIR_IDS[0])
                .unwrap();

            let mut buffer = vec![F::zero(); buffer_size as usize];

            let num_rows = pctx
                .pilout
                .get_air(RANGE_CHECK_4_AIRGROUP_ID, RANGE_CHECK_4_AIR_IDS[0])
                .num_rows();
            let mut trace =
                RangeCheck40Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize)
                    .unwrap();

            let range1 = (BigInt::from(0), BigInt::from((1 << 16) - 1));
            let range2 = (BigInt::from(0), BigInt::from((1 << 8) - 1));
            let range3 = (BigInt::from(50), BigInt::from((1 << 7) - 1));
            let range4 = (BigInt::from(127), BigInt::from(1 << 8));
            let range5 = (BigInt::from(1), BigInt::from((1 << 16) + 1));
            let range6 = (BigInt::from(127), BigInt::from(1 << 16));
            let range7 = (BigInt::from(-1), BigInt::from(1 << 3));
            let range8 = (BigInt::from(-(1 << 7) + 1), BigInt::from(-50));
            let range9 = (BigInt::from(-(1 << 8) + 1), BigInt::from(-127));

            for i in 0..num_rows {
                let selected1 = true; //rng.gen_bool(0.5);
                trace[i].sel1 = F::from_bool(selected1);

                // selected1 and selected2 have to be disjoint for the range check to pass
                let selected2 = false; //if selected1 { false } else { rng.gen_bool(0.5) };
                trace[i].sel2 = F::from_bool(selected2);

                if selected1 {
                    trace[i].a1 = F::from_canonical_u32(rng.gen_range(0..=(1 << 16) - 1));
                    trace[i].a5 = F::from_canonical_u32(rng.gen_range(127..=(1 << 16)));
                    let mut a6_val: i128 = rng.gen_range(-1..=2i128.pow(3));
                    if a6_val < 0 {
                        a6_val = F::order().to_i128().unwrap() + a6_val;
                    }
                    trace[i].a6 = F::from_canonical_u64(a6_val as u64);

                    self.std_lib
                        .range_check(trace[i].a1, range1.0.clone(), range1.1.clone());
                    self.std_lib
                        .range_check(trace[i].a5, range6.0.clone(), range6.1.clone());
                    self.std_lib
                        .range_check(trace[i].a6, range7.0.clone(), range7.1.clone());
                }
                if selected2 {
                    trace[i].a1 = F::from_canonical_u8(rng.gen_range(0..=(1 << 8) - 1));
                    trace[i].a2 = F::from_canonical_u8(rng.gen_range(50..=(1 << 7) - 1));
                    trace[i].a3 = F::from_canonical_u16(rng.gen_range(127..=(1 << 8)));
                    trace[i].a4 = F::from_canonical_u32(rng.gen_range(1..=(1 << 16) + 1));

                    self.std_lib
                        .range_check(trace[i].a1, range2.0.clone(), range2.1.clone());
                    self.std_lib
                        .range_check(trace[i].a2, range3.0.clone(), range3.1.clone());
                    self.std_lib
                        .range_check(trace[i].a3, range4.0.clone(), range4.1.clone());
                    self.std_lib
                        .range_check(trace[i].a4, range5.0.clone(), range5.1.clone());
                }

                let mut a7_val: i128 = rng.gen_range(-2i128.pow(7) + 1..=-50);
                if a7_val < 0 {
                    a7_val = F::order().to_i128().unwrap() + a7_val;
                }
                trace[i].a7 = F::from_canonical_u64(a7_val as u64);
                self.std_lib
                    .range_check(trace[i].a7, range8.0.clone(), range8.1.clone());

                let mut a8_val: i128 = rng.gen_range(-2i128.pow(8) + 1..=-127);
                if a8_val < 0 {
                    a8_val = F::order().to_i128().unwrap() + a8_val;
                }
                trace[i].a8 = F::from_canonical_u64(a8_val as u64);
                self.std_lib
                    .range_check(trace[i].a8, range9.0.clone(), range9.1.clone());
            }

            let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
            air_instance.buffer = buffer;
        }

        self.std_lib.unregister_predecessor(pctx, None);

        log::info!(
            "{}: Completed witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            "RangeCheck4",
            stage
        );
    }
}
