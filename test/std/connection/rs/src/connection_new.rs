use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{ConnectionNew2Trace, CONNECTION_AIRGROUP_ID, CONNECTION_NEW_AIR_IDS};

pub struct ConnectionNew<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField> ConnectionNew<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "ConnectionNew";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let connection_new = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(
            connection_new.clone(),
            Some(CONNECTION_AIRGROUP_ID),
            Some(CONNECTION_NEW_AIR_IDS),
        );

        connection_new
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, _sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Connection".into(), CONNECTION_NEW_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(
            CONNECTION_AIRGROUP_ID,
            CONNECTION_NEW_AIR_IDS[0],
            None,
            buffer,
        );
        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField> WitnessComponent<F> for ConnectionNew<F>
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
            let (_buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("Connection".into(), CONNECTION_NEW_AIR_IDS[0])
                .unwrap();

            let buffer = &mut air_instance.buffer;

            let num_rows = pctx
                .pilout
                .get_air(CONNECTION_AIRGROUP_ID, CONNECTION_NEW_AIR_IDS[0])
                .num_rows();
            let mut trace = ConnectionNew2Trace::map_buffer(
                buffer.as_mut_slice(),
                num_rows,
                offsets[0] as usize,
            )
            .unwrap();

            let mut frame = [0; 6];
            let mut conn_len = [0; 6];
            for i in 0..num_rows {
                // Start connection
                trace[i].a[0] = rng.gen();
                trace[i].b[0] = rng.gen();
                trace[i].c[0] = rng.gen();

                // Start connection
                trace[i].a[1] = rng.gen();
                trace[i].b[1] = rng.gen();
                trace[i].c[1] = rng.gen();
                if i == 3 + frame[1] {
                    trace[i - 1].c[1] = trace[i].c[1];
                    frame[1] += num_rows / 2;
                }

                // TODO: Finish!
                // // Start connection
                // trace[i].a[2] = rng.gen();
                // trace[i].b[2] = rng.gen();
                // trace[i].c[2] = rng.gen();
                // if i == 3 + frame[2] {
                //     trace[i - 1].c[2] = trace[i].c[2];

                //     trace[0 + frame[2]].c[2] = trace[i].b[2];
                //     trace[1 + frame[2]].a[2] = trace[i].b[2];
                //     conn_len[2] += 2;
                // }

                // if i == 3 + frame[2] {
                //     trace[i - 1].c[2] = trace[i].c[2];

                //     trace[0 + frame[2]].c[2] = trace[i].b[2];
                //     trace[1 + frame[2]].a[2] = trace[i].b[2];
                //     conn_len[2] += 2;
                // }

                // if conn_len[2] == 3 {
                //     frame[2] += num_rows / 4;
                //     conn_len[2] = 0;
                // }

                // Start connection
                trace[i].a[3] = rng.gen();
                trace[i].b[3] = rng.gen();
                trace[i].c[3] = rng.gen();
                if i == 2 + frame[3] {
                    trace[i - 1].c[3] = trace[i].a[3];
                    frame[3] += num_rows / 2;
                }

                if i == 3 {
                    trace[i - 3].c[3] = trace[i].b[3];
                    trace[i - 2].a[3] = trace[i - 3].c[3];
                }

                // Start connection
                trace[i].a[4] = rng.gen();
                trace[i].b[4] = rng.gen();
                trace[i].c[4] = rng.gen();

                if i == 2 + frame[4] {
                    trace[i - 1].d[4] = trace[i - 1].b[4];
                    trace[i - 1].a[4] = trace[i].c[4];
                    conn_len[4] += 1;
                }

                if i == 3 + frame[4] {
                    trace[i - 1].b[4] = trace[i].a[4];
                    trace[i].c[4] = trace[i - 1].b[4];
                    conn_len[4] += 1;
                }

                if conn_len[4] == 2 {
                    frame[4] += num_rows / 2;
                    conn_len[4] = 0;
                }

                // Start connection
                trace[i].a[5] = rng.gen();
                trace[i].b[5] = rng.gen();
                trace[i].c[5] = rng.gen();
                if i == 3 + frame[5] {
                    trace[i - 1].d[5] = trace[i].d[5];
                    trace[i - 3].b[5] = trace[i].d[5];
                    conn_len[5] += 2;
                }

                if i == 8 {
                    trace[5].b[5] = trace[i].c[5];
                    trace[1].a[5] = trace[i].c[5];
                }

                if conn_len[5] == 2 {
                    frame[5] += num_rows / 2;
                    conn_len[5] = 0;
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
