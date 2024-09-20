use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;

use crate::{Permutation2_63Trace, PERMUTATION_2_6_AIR_IDS, PERMUTATION_AIRGROUP_ID};

pub struct Permutation2<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Permutation2<F> {
    const MY_NAME: &'static str = "Permutation2";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let permutation2 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(
            permutation2.clone(),
            Some(PERMUTATION_AIRGROUP_ID),
            Some(PERMUTATION_2_6_AIR_IDS),
        );

        permutation2
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Permutation".into(), PERMUTATION_2_6_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(
            PERMUTATION_AIRGROUP_ID,
            PERMUTATION_2_6_AIR_IDS[0],
            None,
            buffer,
        );
        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Permutation2<F> {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
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
            let mut trace = Permutation2_63Trace::map_buffer(
                buffer.as_mut_slice(),
                num_rows,
                offsets[0] as usize,
            )
            .unwrap();

            // Note: Here it is assumed that num_rows of permutation2 is equal to
            //       the sum of num_rows of each variant of permutation1.
            //       Ohterwise, the permutation check cannot be satisfied.
            // Proves
            for i in 0..num_rows {
                trace[i].c1 = F::from_canonical_u8(200);
                trace[i].d1 = F::from_canonical_u8(201);

                trace[i].c2 = F::from_canonical_u8(100);
                trace[i].d2 = F::from_canonical_u8(101);

                trace[i].sel = F::from_bool(true);
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
