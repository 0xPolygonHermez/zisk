use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{Lookup21Trace, LOOKUP_2_AIR_IDS, LOOKUP_SUBPROOF_ID};

pub struct Lookup2<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Lookup2<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Lookup";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let lookup2 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(lookup2.clone(), Some(LOOKUP_2_AIR_IDS));

        lookup2
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Lookup".into(), LOOKUP_2_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(LOOKUP_SUBPROOF_ID[0], LOOKUP_2_AIR_IDS[0], Some(buffer));
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup2<F>
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
            let (buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("Lookup".into(), LOOKUP_2_AIR_IDS[0])
                .unwrap();

            let mut buffer = vec![F::zero(); buffer_size as usize];

            let num_rows = pctx
                .pilout
                .get_air(LOOKUP_SUBPROOF_ID[0], LOOKUP_2_AIR_IDS[0])
                .num_rows();
            let mut trace =
                Lookup21Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize).unwrap();

            for i in 0..num_rows {
                trace[i].a1 = rng.gen();
                trace[i].b1 = rng.gen();
                trace[i].c1 = trace[i].a1;
                trace[i].d1 = trace[i].b1;
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
