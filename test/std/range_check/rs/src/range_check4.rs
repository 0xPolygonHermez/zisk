use std::sync::Arc;

use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{RangeCheck40Trace, RANGE_CHECK_4_AIR_IDS, RANGE_CHECK_4_AIRGROUP_ID};

pub struct RangeCheck4<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField + Copy> RangeCheck4<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "RangeCheck4";

    pub fn new(wcm: &mut WitnessManager<F>, std_lib: Arc<Std<F>>) -> Arc<Self> {
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

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
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

impl<F: PrimeField + Copy> WitnessComponent<F> for RangeCheck4<F>
where
    Standard: Distribution<F>,
{
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
        let mut rng = rand::thread_rng();

        {
            let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
            let air = pctx
                .pilout
                .get_air(air_instance.airgroup_id, air_instance.air_id);

            log::info!(
                "{}: Initiating witness computation for AIR '{}' at stage {}",
                Self::MY_NAME,
                air.name().unwrap_or("unknown"),
                stage
            );

            if stage == 1 {
                let (_, offsets) = ectx
                    .buffer_allocator
                    .as_ref()
                    .get_buffer_info("RangeCheck4".into(), RANGE_CHECK_4_AIR_IDS[0])
                    .unwrap();

                let buffer = &mut air_instance.buffer;

                let num_rows = pctx
                    .pilout
                    .get_air(RANGE_CHECK_4_AIRGROUP_ID, RANGE_CHECK_4_AIR_IDS[0])
                    .num_rows();
                let mut trace = RangeCheck40Trace::map_buffer(
                    buffer.as_mut_slice(),
                    num_rows,
                    offsets[0] as usize,
                )
                .unwrap();

                let range1 = (BigInt::from(0), BigInt::from(2u16.pow(16) - 1));
                let range2 = (BigInt::from(0), BigInt::from(2u8.pow(8) - 1));
                let range3 = (BigInt::from(50), BigInt::from(2u8.pow(7) - 1));
                let range4 = (BigInt::from(127), BigInt::from(2u16.pow(8)));
                let range5 = (BigInt::from(1), BigInt::from(2u32.pow(16) + 1));

                for i in 0..num_rows {
                    // trace[i].a5 = F::from_canonical_u16(rng.gen_range(127..=2u16.pow(16)));
                    // trace[i].a6 = F::from_canonical_u16(rng.gen_range(-1..=2u8.pow(3)));
                    // trace[i].a7 = F::from_canonical_u16(rng.gen_range(-2u8.pow(7) + 1..=-50));
                    // trace[i].a8 = F::from_canonical_u16(rng.gen_range(-2u8.pow(8) + 1..=-127));

                    let selected1 = rng.gen_bool(0.5);
                    trace[i].sel1 = F::from_bool(selected1);
                    // selected1 and selected2 have to be disjoint for the range check to pass
                    let selected2 = if selected1 { false } else { rng.gen_bool(0.5) };
                    trace[i].sel2 = F::from_bool(selected2);

                    if selected1 {
                        trace[i].a1 = F::from_canonical_u16(1);
                        self.std_lib
                            .range_check(trace[i].a1, range1.0.clone(), range1.1.clone());
                    }
                    if selected2 {
                        trace[i].a1 = F::from_canonical_u8(rng.gen_range(0..=2u8.pow(8) - 1));
                        trace[i].a2 = F::from_canonical_u8(rng.gen_range(50..=2u8.pow(7) - 1));
                        trace[i].a3 = F::from_canonical_u16(rng.gen_range(127..=2u16.pow(8)));
                        // trace[i].a4 = F::from_canonical_usize(1); //F::from_canonical_u16(rng.gen_range(1..=2u16.pow(16) + 1));

                        self.std_lib
                            .range_check(trace[i].a1, range2.0.clone(), range2.1.clone());
                        self.std_lib
                            .range_check(trace[i].a2, range3.0.clone(), range3.1.clone());
                        self.std_lib
                            .range_check(trace[i].a3, range4.0.clone(), range4.1.clone());
                        // self.std_lib
                        //     .range_check(trace[i].a4, range5.0.clone(), range5.1.clone());
                    }
                }
            }
        }

        self.std_lib.unregister_predecessor(pctx, None);

        // log::info!(
        //     "{}: Completed witness computation for AIR '{}' at stage {}",
        //     Self::MY_NAME,
        //     air.name().unwrap_or("unknown"),
        //     stage
        // );
    }
}
