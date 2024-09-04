use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;

use crate::{Permutation24Trace, PERMUTATION_2_AIR_IDS, PERMUTATION_SUBPROOF_ID};

pub struct Permutation2<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Permutation2<F> {
    const MY_NAME: &'static str = "Permutation";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let permutation2 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(permutation2.clone(), Some(PERMUTATION_2_AIR_IDS));

        permutation2
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Permutation".into(), PERMUTATION_2_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(
            PERMUTATION_SUBPROOF_ID[0],
            PERMUTATION_2_AIR_IDS[0],
            Some(buffer),
        );
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Permutation2<F> {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
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
            let (buffer_size, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("Permutation".into(), air_id)
                .unwrap();

            let buffer = air_instance.buffer.as_mut().unwrap();

            let num_rows = pctx.pilout.get_air(air_group_id, air_id).num_rows();
            let mut trace =
                Permutation24Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize).unwrap();

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
