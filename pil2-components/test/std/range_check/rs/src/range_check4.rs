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
    const MY_NAME: &'static str = "RngChck4";

    pub fn new(wcm: Arc<WitnessManager<F>>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let range_check4 = Arc::new(Self { std_lib });

        wcm.register_component(range_check4.clone(), Some(RANGE_CHECK_4_AIRGROUP_ID), Some(RANGE_CHECK_4_AIR_IDS));

        // Register dependency relations
        range_check4.std_lib.register_predecessor();

        range_check4
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of the air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info(&sctx, RANGE_CHECK_4_AIRGROUP_ID, RANGE_CHECK_4_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance =
            AirInstance::new(sctx.clone(), RANGE_CHECK_4_AIRGROUP_ID, RANGE_CHECK_4_AIR_IDS[0], None, buffer);
        let (is_myne, gid) =
            ectx.dctx.write().unwrap().add_instance(RANGE_CHECK_4_AIRGROUP_ID, RANGE_CHECK_4_AIR_IDS[0], 1);
        if is_myne {
            pctx.air_instance_repo.add_air_instance(air_instance, Some(gid));
        }
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
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        let mut rng = rand::thread_rng();

        log::debug!("{}: ··· Witness computation for AIR '{}' at stage {}", Self::MY_NAME, "RangeCheck4", stage);

        if stage == 1 {
            let (buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info(&sctx, RANGE_CHECK_4_AIRGROUP_ID, RANGE_CHECK_4_AIR_IDS[0])
                .unwrap();

            let mut buffer = vec![F::zero(); buffer_size as usize];

            let num_rows = pctx.pilout.get_air(RANGE_CHECK_4_AIRGROUP_ID, RANGE_CHECK_4_AIR_IDS[0]).num_rows();
            let mut trace =
                RangeCheck40Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize).unwrap();

            let range1 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 16) - 1), Some(true));
            let range2 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 8) - 1), Some(true));
            let range3 = self.std_lib.get_range(BigInt::from(50), BigInt::from((1 << 7) - 1), Some(true));
            let range4 = self.std_lib.get_range(BigInt::from(127), BigInt::from(1 << 8), Some(true));
            let range5 = self.std_lib.get_range(BigInt::from(1), BigInt::from((1 << 16) + 1), Some(true));
            let range6 = self.std_lib.get_range(BigInt::from(127), BigInt::from(1 << 16), Some(true));
            let range7 = self.std_lib.get_range(BigInt::from(-1), BigInt::from(1 << 3), Some(true));
            let range8 = self.std_lib.get_range(BigInt::from(-(1 << 7) + 1), BigInt::from(-50), Some(true));
            let range9 = self.std_lib.get_range(BigInt::from(-(1 << 8) + 1), BigInt::from(-127), Some(true));

            for i in 0..num_rows {
                let selected1 = rng.gen_bool(0.5);
                trace[i].sel1 = F::from_bool(selected1);

                // selected1 and selected2 have to be disjoint for the range check to pass
                let selected2 = if selected1 { false } else { rng.gen_bool(0.5) };
                trace[i].sel2 = F::from_bool(selected2);

                if selected1 {
                    trace[i].a1 = F::from_canonical_u32(rng.gen_range(0..=(1 << 16) - 1));
                    trace[i].a5 = F::from_canonical_u32(rng.gen_range(127..=(1 << 16)));
                    let mut a6_val: i128 = rng.gen_range(-1..=2i128.pow(3));
                    if a6_val < 0 {
                        a6_val += F::order().to_i128().unwrap();
                    }
                    trace[i].a6 = F::from_canonical_u64(a6_val as u64);

                    self.std_lib.range_check(trace[i].a1, F::one(), range1);
                    self.std_lib.range_check(trace[i].a5, F::one(), range6);
                    self.std_lib.range_check(trace[i].a6, F::one(), range7);
                }
                if selected2 {
                    trace[i].a1 = F::from_canonical_u16(rng.gen_range(0..=(1 << 8) - 1));
                    trace[i].a2 = F::from_canonical_u8(rng.gen_range(50..=(1 << 7) - 1));
                    trace[i].a3 = F::from_canonical_u16(rng.gen_range(127..=(1 << 8)));
                    trace[i].a4 = F::from_canonical_u32(rng.gen_range(1..=(1 << 16) + 1));

                    self.std_lib.range_check(trace[i].a1, F::one(), range2);
                    self.std_lib.range_check(trace[i].a2, F::one(), range3);
                    self.std_lib.range_check(trace[i].a3, F::one(), range4);
                    self.std_lib.range_check(trace[i].a4, F::one(), range5);
                }

                let mut a7_val: i128 = rng.gen_range(-(2i128.pow(7)) + 1..=-50);
                if a7_val < 0 {
                    a7_val += F::order().to_i128().unwrap();
                }
                trace[i].a7 = F::from_canonical_u64(a7_val as u64);
                self.std_lib.range_check(trace[i].a7, F::one(), range8);

                let mut a8_val: i128 = rng.gen_range(-(2i128.pow(8)) + 1..=-127);
                if a8_val < 0 {
                    a8_val += F::order().to_i128().unwrap();
                }
                trace[i].a8 = F::from_canonical_u64(a8_val as u64);
                self.std_lib.range_check(trace[i].a8, F::one(), range9);
            }

            let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
            air_instance.buffer = buffer;
        }

        self.std_lib.unregister_predecessor(pctx, None);
    }
}
