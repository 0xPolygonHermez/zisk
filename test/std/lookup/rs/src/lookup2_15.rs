use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{Lookup2_154Trace, LOOKUP_2_15_AIR_IDS, LOOKUP_AIRGROUP_ID};

pub struct Lookup2_15<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Lookup2_15<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Lookup2_15";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let lookup2_15 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(
            lookup2_15.clone(),
            Some(LOOKUP_AIRGROUP_ID),
            Some(LOOKUP_2_15_AIR_IDS),
        );

        lookup2_15
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Lookup".into(), LOOKUP_2_15_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance =
            AirInstance::new(LOOKUP_AIRGROUP_ID, LOOKUP_2_15_AIR_IDS[0], None, buffer);

        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup2_15<F>
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
            "{}: Initiating witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );

        if stage == 1 {
            let (_, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("Lookup".into(), LOOKUP_2_15_AIR_IDS[0])
                .unwrap();

            let buffer = &mut air_instance.buffer;

            let num_rows = pctx
                .pilout
                .get_air(LOOKUP_AIRGROUP_ID, LOOKUP_2_15_AIR_IDS[0])
                .num_rows();
            let mut trace =
                Lookup2_154Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize)
                    .unwrap();

            // TODO: Add the ability to send inputs to lookup3
            //       and consequently add random selectors

            for i in 0..num_rows {
                // Inner lookups
                trace[i].a1 = rng.gen();
                trace[i].b1 = rng.gen();
                trace[i].c1 = trace[i].a1;
                trace[i].d1 = trace[i].b1;

                trace[i].a3 = rng.gen();
                trace[i].b3 = rng.gen();
                trace[i].c2 = trace[i].a3;
                trace[i].d2 = trace[i].b3;
                let selected = rng.gen_bool(0.5);
                trace[i].sel1 = F::from_bool(selected);
                if selected {
                    trace[i].mul = trace[i].sel1;
                }

                // Outer lookups
                trace[i].a2 = F::from_canonical_usize(i % (1 << 14));
                trace[i].b2 = F::from_canonical_usize(i % (1 << 14));

                trace[i].a4 = F::from_canonical_usize(i % (1 << 14));
                trace[i].b4 = F::from_canonical_usize(i % (1 << 14));
                trace[i].sel2 = F::from_bool(true);
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
