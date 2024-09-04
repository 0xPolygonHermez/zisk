use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;

use crate::{Lookup00Trace, LOOKUP_0_AIR_IDS, LOOKUP_SUBPROOF_ID};

pub struct Lookup0<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Lookup0<F> {
    const MY_NAME: &'static str = "Lookup";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let lookup0 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(lookup0.clone(), Some(LOOKUP_0_AIR_IDS));

        lookup0
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Lookup".into(), LOOKUP_0_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(LOOKUP_SUBPROOF_ID[0], LOOKUP_0_AIR_IDS[0], Some(buffer));
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup0<F> {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        // let mut rng = rand::thread_rng();

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
                .get_buffer_info("Lookup".into(), LOOKUP_0_AIR_IDS[0])
                .unwrap();

            let buffer = air_instance.buffer.as_mut().unwrap();

            let num_rows = pctx
                .pilout
                .get_air(LOOKUP_SUBPROOF_ID[0], LOOKUP_0_AIR_IDS[0])
                .num_rows();
            let mut trace =
                Lookup00Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize)
                    .unwrap();

            let num_lookups = trace[0].sel.len();

            for i in 0..num_rows {
                for j in 0..num_lookups {
                    trace[i].f[2 * j] = F::from_canonical_usize(i);
                    trace[i].f[2 * j + 1] = F::from_canonical_usize(i);
                    trace[i].sel[j] = F::from_bool(true);
                    trace[i].t[2 * j] = F::from_canonical_usize(i);
                    trace[i].t[2 * j + 1] = F::from_canonical_usize(i);
                    trace[i].mul[j] = F::from_canonical_usize(2);
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
