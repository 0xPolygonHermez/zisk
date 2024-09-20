use std::sync::Arc;

use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{MultiRangeCheck20Trace, MULTI_RANGE_CHECK_2_AIRGROUP_ID, MULTI_RANGE_CHECK_2_AIR_IDS};

pub struct MultiRangeCheck2<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> MultiRangeCheck2<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "MultiRangeCheck2";

    pub fn new(wcm: Arc< WitnessManager<F>>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let multi_range_check2 = Arc::new(Self { std_lib });

        wcm.register_component(
            multi_range_check2.clone(),
            Some(MULTI_RANGE_CHECK_2_AIRGROUP_ID),
            Some(MULTI_RANGE_CHECK_2_AIR_IDS),
        );

        // Register dependency relations
        multi_range_check2.std_lib.register_predecessor();

        multi_range_check2
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,) {
        // For simplicity, add a single instance of the air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("MultiRangeCheck2".into(), MULTI_RANGE_CHECK_2_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(
            MULTI_RANGE_CHECK_2_AIRGROUP_ID,
            MULTI_RANGE_CHECK_2_AIR_IDS[0],
            None,
            buffer,
        );
        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField> WitnessComponent<F> for MultiRangeCheck2<F>
where
    Standard: Distribution<F>,
{
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
        let mut rng = rand::thread_rng();

        log::info!(
            "{}: Initiating witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            "MultiRangeCheck2",
            stage
        );

        if stage == 1 {
            let (buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("MultiRangeCheck2".into(), MULTI_RANGE_CHECK_2_AIR_IDS[0])
                .unwrap();

            let mut buffer = vec![F::zero(); buffer_size as usize];

            let num_rows = pctx
                .pilout
                .get_air(
                    MULTI_RANGE_CHECK_2_AIRGROUP_ID,
                    MULTI_RANGE_CHECK_2_AIR_IDS[0],
                )
                .num_rows();
            let mut trace = MultiRangeCheck20Trace::map_buffer(
                buffer.as_mut_slice(),
                num_rows,
                offsets[0] as usize,
            )
            .unwrap();

            let range1 = (BigInt::from(1 << 5), BigInt::from((1 << 8) - 1));
            let range2 = (BigInt::from(1 << 8), BigInt::from((1 << 9) - 1));
            let range3 = (BigInt::from(0), BigInt::from((1 << 7) - 1));
            let range4 = (BigInt::from(0), BigInt::from((1 << 4) - 1));

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

                        self.std_lib
                            .range_check(trace[i].a[0], range1.0.clone(), range1.1.clone());
                    } else {
                        trace[i].a[0] = F::from_canonical_u16(rng.gen_range((1 << 8)..=(1 << 9) - 1));

                        self.std_lib
                            .range_check(trace[i].a[0], range2.0.clone(), range2.1.clone());
                    }
                }

                if selected2 {
                    if range_selector2 {
                        trace[i].a[1] = F::from_canonical_u16(rng.gen_range(0..=(1 << 7) - 1));

                        self.std_lib
                            .range_check(trace[i].a[1], range3.0.clone(), range3.1.clone());
                    } else {
                        trace[i].a[1] = F::from_canonical_u16(rng.gen_range(0..=(1 << 4) - 1));

                        self.std_lib
                            .range_check(trace[i].a[1], range4.0.clone(), range4.1.clone());
                    }
                }
            }

            let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
            air_instance.buffer = buffer;
        }

        self.std_lib.unregister_predecessor(pctx, None);

        log::info!(
            "{}: Completed witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            "MultiRangeCheck2",
            stage
        );
    }
}
