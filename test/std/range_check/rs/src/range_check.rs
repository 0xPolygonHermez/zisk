use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use pil_std_lib::Std;

use p3_field::PrimeField;
use rand::Rng;
use num_bigint::BigInt;

use crate::{RangeCheck10Trace, RANGE_CHECK_1_SUBPROOF_ID, RANGE_CHECK_1_AIR_IDS};

pub struct RangeCheck<F> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField + Copy> RangeCheck<F> {
    const MY_NAME: &'static str = "RangeCheck";

    pub fn new(wcm: &mut WitnessManager<F>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let range_check = Arc::new(Self { std_lib });

        wcm.register_component(range_check.clone(), Some(RANGE_CHECK_1_AIR_IDS));

        range_check
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of the air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("RangeCheck1".into(), RANGE_CHECK_1_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(
            RANGE_CHECK_1_SUBPROOF_ID[0],
            RANGE_CHECK_1_AIR_IDS[0],
            Some(buffer),
        );
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for RangeCheck<F> {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        // let mut rng = rand::thread_rng();

        let air_instances_vec = &mut pctx.air_instances.write().unwrap();
        let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
        let air = pctx.pilout.get_air(air_instance.air_group_id, air_instance.air_id);

        log::info!(
            "{}: Initiating witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );

        if stage == 1 {
            let (buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("RangeCheck1".into(), RANGE_CHECK_1_AIR_IDS[0])
                .unwrap();

            let mut buffer = vec![F::zero(); buffer_size as usize];

            let num_rows = pctx.pilout.get_air(RANGE_CHECK_1_SUBPROOF_ID[0], RANGE_CHECK_1_AIR_IDS[0]).num_rows();
            let mut trace = RangeCheck10Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize).unwrap();

            for i in 0..num_rows {
                // TODO: Do it with real random values
                // a1[i] = getRandom(0, 2**8-1);
                // a2[i] = getRandom(0, 2**4-1);
                // a3[i] = getRandom(60, 2**16-1);
                // a4[i] = getRandom(8228, 17400);
                // a5[i] = getRandom(0, 2**8-1);

                // sel1[i] = getRandom(0, 1);
                // sel2[i] = getRandom(0, 1);
                // sel3[i] = getRandom(0, 1);

                trace[i].a1 = F::from_canonical_u16(0);
                trace[i].a2 = F::from_canonical_u16(0);
                trace[i].a3 = F::from_canonical_u16(60);
                trace[i].a4 = F::from_canonical_u16(8228);
                trace[i].a5 = F::from_canonical_u16(0);
        
                trace[i].sel1 = F::from_bool(true);
                trace[i].sel2 = F::from_bool(true);
                trace[i].sel3 = F::from_bool(true);

                // TODO: We have to redo it to avoid that many type conversions
                if trace[i].sel1.is_one() {
                    self.std_lib.range_check(BigInt::from(trace[i].a1.as_canonical_biguint()), BigInt::from(0), BigInt::from(2u16.pow(8) - 1));
                    self.std_lib.range_check(BigInt::from(trace[i].a3.as_canonical_biguint()), BigInt::from(60), BigInt::from(2u16.pow(16) - 1));
                }
                if trace[i].sel2.is_one() {
                    self.std_lib.range_check(BigInt::from(trace[i].a2.as_canonical_biguint()), BigInt::from(0), BigInt::from(2u16.pow(4) - 1));
                    self.std_lib.range_check(BigInt::from(trace[i].a4.as_canonical_biguint()), BigInt::from(8228), BigInt::from(17400));
                }
                if trace[i].sel3.is_one() {
                    self.std_lib.range_check(BigInt::from(trace[i].a5.as_canonical_biguint()), BigInt::from(0), BigInt::from(2u16.pow(8) - 1));
                }
            }
        }

        log::info!(
            "{}: Completed witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );
    }
}
