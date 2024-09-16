use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{Permutation1_82Trace, PERMUTATION_1_8_AIR_IDS, PERMUTATION_AIRGROUP_ID};

pub struct Permutation1_8<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Permutation1_8<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Permutation1_8";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let permutation1_8 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(
            permutation1_8.clone(),
            Some(PERMUTATION_AIRGROUP_ID[0]),
            Some(PERMUTATION_1_8_AIR_IDS),
        );

        permutation1_8
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Permutation".into(), PERMUTATION_1_8_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(
            PERMUTATION_AIRGROUP_ID[0],
            PERMUTATION_1_8_AIR_IDS[0],
            None,
            buffer,
        );
        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Permutation1_8<F>
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

        let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];

        let airgroup_id = air_instance.airgroup_id;
        let air_id = air_instance.air_id;
        let air = pctx.pilout.get_air(airgroup_id, air_id);

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
                .get_buffer_info("Permutation".into(), air_id)
                .unwrap();

            let buffer = &mut air_instance.buffer;
            let num_rows = pctx.pilout.get_air(airgroup_id, air_id).num_rows();

            // I cannot, programatically, link the permutation trace with its air_id
            let mut trace = Permutation1_82Trace::map_buffer(
                buffer.as_mut_slice(),
                num_rows,
                offsets[0] as usize,
            )
            .unwrap();

            // TODO: Add the ability to send inputs to permutation2
            //       and consequently add random selectors

            // Assumes
            for i in 0..num_rows {
                trace[i].a1 = rng.gen();
                trace[i].b1 = rng.gen();

                trace[i].a2 = F::from_canonical_u8(200);
                trace[i].b2 = F::from_canonical_u8(201);

                trace[i].a3 = rng.gen();
                trace[i].b3 = rng.gen();

                trace[i].a4 = F::from_canonical_u8(100);
                trace[i].b4 = F::from_canonical_u8(101);

                trace[i].sel1 = F::one();
                trace[i].sel3 = F::one(); // F::from_canonical_u8(rng.gen_range(0..=1));
            }

            // TODO: Add the permutation of indexes

            // Proves
            for i in 0..num_rows {
                let index = num_rows - i - 1;
                // let mut index = rng.gen_range(0..num_rows);
                trace[i].c1 = trace[index].a1;
                trace[i].d1 = trace[index].b1;

                // index = rng.gen_range(0..num_rows);
                trace[i].c2 = trace[index].a3;
                trace[i].d2 = trace[index].b3;

                trace[i].sel2 = trace[i].sel1;
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
