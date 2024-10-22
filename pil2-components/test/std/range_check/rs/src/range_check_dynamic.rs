use std::sync::Arc;

use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{RangeCheckDynamic0Trace, RANGE_CHECK_DYNAMIC_AIRGROUP_ID, RANGE_CHECK_DYNAMIC_AIR_IDS};

pub struct RangeCheckDynamic<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> RangeCheckDynamic<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "RngChDy ";

    pub fn new(wcm: Arc<WitnessManager<F>>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let multi_range_check1 = Arc::new(Self { std_lib });

        wcm.register_component(
            multi_range_check1.clone(),
            Some(RANGE_CHECK_DYNAMIC_AIRGROUP_ID),
            Some(RANGE_CHECK_DYNAMIC_AIR_IDS),
        );

        // Register dependency relations
        multi_range_check1.std_lib.register_predecessor();

        multi_range_check1
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of the air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info(&sctx, RANGE_CHECK_DYNAMIC_AIRGROUP_ID, RANGE_CHECK_DYNAMIC_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance =
            AirInstance::new(RANGE_CHECK_DYNAMIC_AIRGROUP_ID, RANGE_CHECK_DYNAMIC_AIR_IDS[0], None, buffer);
        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField> WitnessComponent<F> for RangeCheckDynamic<F>
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

        log::debug!("{}: ··· Witness computation for AIR '{}' at stage {}", Self::MY_NAME, "RangeCheckDynamic", stage);

        if stage == 1 {
            let (buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info(&sctx, RANGE_CHECK_DYNAMIC_AIRGROUP_ID, RANGE_CHECK_DYNAMIC_AIR_IDS[0])
                .unwrap();

            let mut buffer = vec![F::zero(); buffer_size as usize];

            let num_rows =
                pctx.pilout.get_air(RANGE_CHECK_DYNAMIC_AIRGROUP_ID, RANGE_CHECK_DYNAMIC_AIR_IDS[0]).num_rows();
            let mut trace =
                RangeCheckDynamic0Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize).unwrap();

            let range7 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 7) - 1), Some(false));
            let range8 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 8) - 1), Some(false));
            let range16 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 16) - 1), Some(false));
            let range17 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 17) - 1), Some(false));

            for i in 0..num_rows {
                let range = rng.gen_range(0..=3);

                match range {
                    0 => {
                        trace[i].sel_7 = F::one();
                        trace[i].colu = F::from_canonical_u16(rng.gen_range(0..=(1 << 7) - 1));

                        self.std_lib.range_check(trace[i].colu, F::one(), range7);
                    }
                    1 => {
                        trace[i].sel_8 = F::one();
                        trace[i].colu = F::from_canonical_u16(rng.gen_range(0..=(1 << 8) - 1));

                        self.std_lib.range_check(trace[i].colu, F::one(), range8);
                    }
                    2 => {
                        trace[i].sel_16 = F::one();
                        trace[i].colu = F::from_canonical_u32(rng.gen_range(0..=(1 << 16) - 1));

                        self.std_lib.range_check(trace[i].colu, F::one(), range16);
                    }
                    3 => {
                        trace[i].sel_17 = F::one();
                        trace[i].colu = F::from_canonical_u32(rng.gen_range(0..=(1 << 17) - 1));

                        self.std_lib.range_check(trace[i].colu, F::one(), range17);
                    }
                    _ => panic!("Invalid range"),
                }
            }

            let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
            air_instance.buffer = buffer;
        }

        self.std_lib.unregister_predecessor(pctx, None);
    }
}
