use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{Lookup00Trace, LOOKUP_0_AIR_IDS, LOOKUP_AIRGROUP_ID};

pub struct Lookup0<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Lookup0<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Lookup0";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let lookup0 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(
            lookup0.clone(),
            Some(LOOKUP_AIRGROUP_ID),
            Some(LOOKUP_0_AIR_IDS),
        );

        lookup0
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info(&sctx, LOOKUP_AIRGROUP_ID, LOOKUP_0_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(LOOKUP_AIRGROUP_ID, LOOKUP_0_AIR_IDS[0], None, buffer);

        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup0<F>
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
        let mut rng = rand::thread_rng();

        let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];
        let air = pctx
            .pilout
            .get_air(air_instance.airgroup_id, air_instance.air_id);

        log::info!(
            "{}: ··· Witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );

        if stage == 1 {
            let (_buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info(&sctx, LOOKUP_AIRGROUP_ID, LOOKUP_0_AIR_IDS[0])
                .unwrap();

            let buffer = &mut air_instance.buffer;

            let num_rows = pctx
                .pilout
                .get_air(LOOKUP_AIRGROUP_ID, LOOKUP_0_AIR_IDS[0])
                .num_rows();
            let mut trace =
                Lookup00Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize)
                    .unwrap();

            let num_lookups = trace[0].sel.len();

            for j in 0..num_lookups {
                for i in 0..num_rows {
                    // Assumes
                    trace[i].f[2 * j] = rng.gen();
                    trace[i].f[2 * j + 1] = rng.gen();
                    let selected = rng.gen_bool(0.5);
                    trace[i].sel[j] = F::from_bool(selected);

                    // Proves
                    trace[i].t[2 * j] = trace[i].f[2 * j];
                    trace[i].t[2 * j + 1] = trace[i].f[2 * j + 1];
                    if selected {
                        trace[i].mul[j] = F::one();
                    }
                }
            }
        }
    }
}
