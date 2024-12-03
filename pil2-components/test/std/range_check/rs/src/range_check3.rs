use std::sync::Arc;

use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use num_bigint::BigInt;
use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{RangeCheck3Trace, RANGE_CHECK_3_AIRGROUP_ID, RANGE_CHECK_3_AIR_IDS};

pub struct RangeCheck3<F: PrimeField> {
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> RangeCheck3<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "RngChck3";

    pub fn new(wcm: Arc<WitnessManager<F>>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let range_check1 = Arc::new(Self { std_lib });

        wcm.register_component(range_check1.clone(), Some(RANGE_CHECK_3_AIRGROUP_ID), Some(RANGE_CHECK_3_AIR_IDS));

        // Register dependency relations
        range_check1.std_lib.register_predecessor();

        range_check1
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let num_rows = pctx.global_info.airs[RANGE_CHECK_3_AIRGROUP_ID][RANGE_CHECK_3_AIR_IDS[0]].num_rows;
        let trace = RangeCheck3Trace::new(num_rows);

        let air_instance = AirInstance::new(
            sctx.clone(),
            RANGE_CHECK_3_AIRGROUP_ID,
            RANGE_CHECK_3_AIR_IDS[0],
            None,
            trace.buffer.unwrap(),
        );
        let (is_myne, gid) =
            ectx.dctx.write().unwrap().add_instance(RANGE_CHECK_3_AIRGROUP_ID, RANGE_CHECK_3_AIR_IDS[0], 1);
        if is_myne {
            pctx.air_instance_repo.add_air_instance(air_instance, Some(gid));
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for RangeCheck3<F>
where
    Standard: Distribution<F>,
{
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
        let mut rng = rand::thread_rng();

        log::debug!("{}: ··· Witness computation for AIR '{}' at stage {}", Self::MY_NAME, "RangeCheck3", stage);

        if stage == 1 {
            let proof_ctx = pctx.clone();
            let air_instances_vec = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
            let buffer = &mut air_instance.trace;

            let num_rows = pctx.pilout.get_air(RANGE_CHECK_3_AIRGROUP_ID, RANGE_CHECK_3_AIR_IDS[0]).num_rows();
            let mut trace = RangeCheck3Trace::map_buffer(buffer.as_mut_slice(), num_rows, 0).unwrap();

            let range1 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 4) - 1), Some(false));
            let range2 = self.std_lib.get_range(BigInt::from(0), BigInt::from((1 << 8) - 1), Some(false));

            for i in 0..num_rows {
                trace[i].c1 = F::from_canonical_u16(rng.gen_range(0..=(1 << 4) - 1));
                trace[i].c2 = F::from_canonical_u16(rng.gen_range(0..=(1 << 8) - 1));

                self.std_lib.range_check(trace[i].c1, F::one(), range1);
                self.std_lib.range_check(trace[i].c2, F::one(), range2);
            }
        }

        self.std_lib.unregister_predecessor(pctx, None);
    }
}
