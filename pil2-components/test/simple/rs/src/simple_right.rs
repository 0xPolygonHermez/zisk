use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{SimpleRight1Trace, SIMPLE_AIRGROUP_ID, SIMPLE_RIGHT_AIR_IDS};

pub struct SimpleRight<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> SimpleRight<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "SimRight";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let simple_right = Arc::new(Self { _phantom: std::marker::PhantomData });

        wcm.register_component(simple_right.clone(), Some(SIMPLE_AIRGROUP_ID), Some(SIMPLE_RIGHT_AIR_IDS));

        simple_right
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let (buffer_size, _) =
            ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, SIMPLE_AIRGROUP_ID, SIMPLE_RIGHT_AIR_IDS[0]).unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(sctx.clone(), SIMPLE_AIRGROUP_ID, SIMPLE_RIGHT_AIR_IDS[0], None, buffer);
        pctx.air_instance_repo.add_air_instance(air_instance, None);
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for SimpleRight<F>
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
        let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];

        let airgroup_id = air_instance.airgroup_id;
        let air_id = air_instance.air_id;
        let air = pctx.pilout.get_air(airgroup_id, air_id);

        log::debug!(
            "{}: ··· Computing witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );

        if stage == 1 {
            let (_, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info(&sctx, SIMPLE_AIRGROUP_ID, SIMPLE_RIGHT_AIR_IDS[0])
                .unwrap();

            let buffer = &mut air_instance.buffer;
            let num_rows = pctx.pilout.get_air(airgroup_id, air_id).num_rows();

            // I cannot, programatically, link the permutation trace with its air_id
            let mut trace =
                SimpleRight1Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize).unwrap();

            // Proves
            for i in 0..num_rows {
                trace[i].a = F::from_canonical_u8(200);
                trace[i].b = F::from_canonical_u8(201);

                trace[i].c = F::from_canonical_usize(i);
                trace[i].d = F::from_canonical_usize(num_rows - i - 1);

                trace[i].mul = F::from_canonical_usize(1);
            }
        }
    }
}
