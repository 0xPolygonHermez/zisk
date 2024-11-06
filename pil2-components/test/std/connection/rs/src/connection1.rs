use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{Connection1Trace, CONNECTION_1_AIR_IDS, CONNECTION_AIRGROUP_ID};

pub struct Connection1<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Connection1<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Connct_1";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let connection1 = Arc::new(Self { _phantom: std::marker::PhantomData });

        wcm.register_component(connection1.clone(), Some(CONNECTION_AIRGROUP_ID), Some(CONNECTION_1_AIR_IDS));

        connection1
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx<F>>, sctx: Arc<SetupCtx<F>>) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info(&sctx, CONNECTION_AIRGROUP_ID, CONNECTION_1_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance =
            AirInstance::new(sctx.clone(), CONNECTION_AIRGROUP_ID, CONNECTION_1_AIR_IDS[0], None, buffer);
        let (is_myne, gid) =
            ectx.dctx.write().unwrap().add_instance(CONNECTION_AIRGROUP_ID, CONNECTION_1_AIR_IDS[0], 1);
        if is_myne {
            pctx.air_instance_repo.add_air_instance(air_instance, Some(gid));
        }
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Connection1<F>
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

        let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
        let air = pctx.pilout.get_air(air_instance.airgroup_id, air_instance.air_id);

        log::debug!(
            "{}: ··· Witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );

        if stage == 1 {
            let (_buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info(&sctx, CONNECTION_AIRGROUP_ID, CONNECTION_1_AIR_IDS[0])
                .unwrap();

            let buffer = &mut air_instance.buffer;

            let num_rows = pctx.pilout.get_air(CONNECTION_AIRGROUP_ID, CONNECTION_1_AIR_IDS[0]).num_rows();
            let mut trace =
            Connection1Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize).unwrap();

            for i in 0..num_rows {
                trace[i].a = rng.gen();
                trace[i].b = rng.gen();
                trace[i].c = rng.gen();
            }
        }
    }
}
