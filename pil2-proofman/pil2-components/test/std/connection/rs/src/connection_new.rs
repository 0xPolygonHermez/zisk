use std::sync::Arc;

use witness::WitnessComponent;
use proofman_common::{add_air_instance, FromTrace, AirInstance, ProofCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::ConnectionNewTrace;

pub struct ConnectionNew;

impl ConnectionNew {
    const MY_NAME: &'static str = "Connct_N";

    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl<F: PrimeField> WitnessComponent<F> for ConnectionNew
where
    Standard: Distribution<F>,
{
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        let mut rng = rand::thread_rng();
        let mut trace = ConnectionNewTrace::new_zeroes();
        let num_rows = trace.num_rows();

        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

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

        let air_instance = AirInstance::new_from_trace(FromTrace::new(&mut trace));
        add_air_instance::<F>(air_instance, pctx.clone());
    }
}
