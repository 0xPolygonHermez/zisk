use std::sync::Arc;

use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{RangeCheck1Trace, RANGE_CHECK_1_AIRGROUP_ID, RANGE_CHECK_1_AIR_IDS};

pub struct RangeCheck1<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> RangeCheck1<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "RngChck1";

    pub fn new(wcm: Arc<WitnessManager<F>>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let range_check1 = Arc::new(Self { std_lib });

        wcm.register_component(range_check1.clone(), Some(RANGE_CHECK_1_AIRGROUP_ID), Some(RANGE_CHECK_1_AIR_IDS));

        // Register dependency relations
        range_check1.std_lib.register_predecessor();

        range_check1
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx<F>>, sctx: Arc<SetupCtx<F>>) {
        // For simplicity, add a single instance of the air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info(&sctx, RANGE_CHECK_1_AIRGROUP_ID, RANGE_CHECK_1_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance =
            AirInstance::new(sctx.clone(), RANGE_CHECK_1_AIRGROUP_ID, RANGE_CHECK_1_AIR_IDS[0], None, buffer);
        let (is_myne, gid) =
            ectx.dctx.write().unwrap().add_instance(RANGE_CHECK_1_AIRGROUP_ID, RANGE_CHECK_1_AIR_IDS[0], 1);
        if is_myne {
            pctx.air_instance_repo.add_air_instance(air_instance, Some(gid));
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for RangeCheck1<F>
where
    Standard: Distribution<F>,
{
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
    ) {
        let mut rng = rand::thread_rng();

        log::debug!("{}: ··· Witness computation for AIR '{}' at stage {}", Self::MY_NAME, "RangeCheck1", stage);

        if stage == 1 {
            let (buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info(&sctx, RANGE_CHECK_1_AIRGROUP_ID, RANGE_CHECK_1_AIR_IDS[0])
                .unwrap();

            let mut buffer = vec![F::zero(); buffer_size as usize];

            let num_rows = pctx.pilout.get_air(RANGE_CHECK_1_AIRGROUP_ID, RANGE_CHECK_1_AIR_IDS[0]).num_rows();
            let mut trace = RangeCheck1Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize).unwrap();

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

            let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
            air_instance.buffer = buffer;
        }

        self.std_lib.unregister_predecessor(pctx, None);
    }
}
