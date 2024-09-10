use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{Connection10Trace, CONNECTION_1_AIR_IDS, CONNECTION_SUBPROOF_ID};

pub struct Connection1<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Connection1<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Connection";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let connection1 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(
            connection1.clone(),
            Some(CONNECTION_SUBPROOF_ID[0]),
            Some(CONNECTION_1_AIR_IDS),
        );

        connection1
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Connection".into(), CONNECTION_1_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(
            CONNECTION_SUBPROOF_ID[0],
            CONNECTION_1_AIR_IDS[0],
            None,
            Some(buffer),
        );
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
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
        let mut rng = rand::thread_rng();

        let air_instances_vec = &mut pctx.air_instances.write().unwrap();
        let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
        let air = pctx
            .pilout
            .get_air(air_instance.air_group_id, air_instance.air_id);

        log::info!(
            "{}: Initiating witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );

        if stage == 1 {
            let (_buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("Connection".into(), CONNECTION_1_AIR_IDS[0])
                .unwrap();

            let buffer = air_instance.buffer.as_mut().unwrap();

            let num_rows = pctx
                .pilout
                .get_air(CONNECTION_SUBPROOF_ID[0], CONNECTION_1_AIR_IDS[0])
                .num_rows();
            let mut trace =
                Connection10Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize)
                    .unwrap();

            for i in 0..num_rows {
                trace[i].a = rng.gen();
                trace[i].b = rng.gen();
                trace[i].c = rng.gen();
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
