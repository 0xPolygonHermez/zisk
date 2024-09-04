use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{ConnectionNew1Trace, CONNECTION_NEW_AIR_IDS, CONNECTION_SUBPROOF_ID};

pub struct ConnectionNew<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField> ConnectionNew<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Connection";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let connection_new = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(connection_new.clone(), Some(CONNECTION_NEW_AIR_IDS));

        connection_new
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Connection".into(), CONNECTION_NEW_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(
            CONNECTION_SUBPROOF_ID[0],
            CONNECTION_NEW_AIR_IDS[0],
            Some(buffer),
        );
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
                .get_buffer_info("Connection".into(), CONNECTION_NEW_AIR_IDS[0])
                .unwrap();

            let buffer = air_instance.buffer.as_mut().unwrap();

            let num_rows = pctx
                .pilout
                .get_air(CONNECTION_SUBPROOF_ID[0], CONNECTION_NEW_AIR_IDS[0])
                .num_rows();
            let mut trace =
                ConnectionNew1Trace::map_buffer(buffer.as_mut_slice(),  num_rows, offsets[0] as usize)
                    .unwrap();

            let mut frame = [0; 5];
            let mut conn_len = [0; 5];
            for i in 0..num_rows {
                // Start connection
                trace[i].a[0] = rng.gen();
                trace[i].b[0] = rng.gen();
                trace[i].c[0] = rng.gen();

                trace[i].a[1] = rng.gen();
                trace[i].b[1] = rng.gen();
                trace[i].c[1] = rng.gen();
                if i == 3 + frame[1] {
                    trace[i - 1].c[1] = trace[i].c[1];
                    frame[1] += num_rows / 2;
                }

                // Start connection
                trace[i].a[2] = rng.gen();
                trace[i].b[2] = rng.gen();
                trace[i].c[2] = rng.gen();
                if i == 2 + frame[2] {
                    trace[i - 1].c[2] = trace[i].a[2];
                    conn_len[2] += 1;
                }

                if i == 3 + frame[2] {
                    trace[0 + frame[2]].c[2] = trace[i].b[2];
                    trace[1 + frame[2]].a[2] = trace[i].b[2];
                    conn_len[2] += 2;
                }

                if conn_len[2] == 3 {
                    frame[2] += num_rows / 2;
                    conn_len[2] = 0;
                }

                // Start connection
                trace[i].a[3] = rng.gen();
                trace[i].b[3] = rng.gen();
                trace[i].c[3] = rng.gen();
                if i == 2 + frame[3] {
                    trace[i - 1].d[3] = trace[i - 1].b[3];
                    trace[i - 1].a[3] = trace[i].c[3];
                    conn_len[3] += 2;
                }

                if i == 3 + frame[3] {
                    trace[i - 1].b[3] = trace[i].a[3];
                    trace[i - 1].c[3] = trace[i].a[3];
                    conn_len[3] += 2;
                }

                if conn_len[3] == 4 {
                    frame[3] += num_rows / 2;
                    conn_len[3] = 0;
                }

                // Start connection
                trace[i].a[4] = rng.gen();
                trace[i].b[4] = rng.gen();
                trace[i].c[4] = rng.gen();
                if (i == 2 + frame[4]) || (i == 3 + frame[4]) {
                    trace[i].d[4] = trace[frame[4]].b[4];
                    conn_len[4] += 2;
                }

                if conn_len[4] == 2 {
                    frame[4] += num_rows / 2;
                    conn_len[4] = 0;
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
