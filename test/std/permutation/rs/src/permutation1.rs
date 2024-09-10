use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, seq::SliceRandom, Rng};

use crate::{Permutation10Trace, PERMUTATION_1_AIR_IDS, PERMUTATION_SUBPROOF_ID};

pub struct Permutation1<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Permutation1<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Permutation1";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let permutation1 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(
            permutation1.clone(),
            Some(PERMUTATION_SUBPROOF_ID[0]),
            Some(PERMUTATION_1_AIR_IDS),
        );

        permutation1
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // Add two instances of this air, so that 2**6 + 2**6 = 2**7 to fit with permutation2
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Permutation".into(), PERMUTATION_1_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(
            PERMUTATION_SUBPROOF_ID[0],
            PERMUTATION_1_AIR_IDS[0],
            None,
            Some(buffer),
        );

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(
            PERMUTATION_SUBPROOF_ID[0],
            PERMUTATION_1_AIR_IDS[0],
            None,
            Some(buffer),
        );
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Permutation1<F>
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

        let air_group_id = air_instance.air_group_id;
        let air_id = air_instance.air_id;
        let air = pctx.pilout.get_air(air_group_id, air_id);

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

            let buffer = air_instance.buffer.as_mut().unwrap();
            let num_rows = pctx.pilout.get_air(air_group_id, air_id).num_rows();

            // I cannot, programatically, link the permutation trace with its air_id
            let mut trace = Permutation10Trace::map_buffer(
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

                trace[i].sel1 = F::from_bool(rng.gen_bool(0.5));
                trace[i].sel3 = F::one();
            }

            let mut indices: Vec<usize> = (0..num_rows).collect();
            indices.shuffle(&mut rng);

            // Proves
            for i in 0..num_rows {
                // We take a random permutation of the indices to show that the permutation check is passing
                trace[i].c1 = trace[indices[i]].a1;
                trace[i].d1 = trace[indices[i]].b1;

                trace[i].c2 = trace[indices[i]].a3;
                trace[i].d2 = trace[indices[i]].b3;

                trace[i].sel2 = trace[indices[i]].sel1;
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
