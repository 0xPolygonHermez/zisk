use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;

use crate::{Lookup33Trace, LOOKUP_3_AIR_IDS, LOOKUP_SUBPROOF_ID};

pub struct Lookup3<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Lookup3<F> {
    const MY_NAME: &'static str = "Lookup3";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let lookup3 = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(
            lookup3.clone(),
            Some(LOOKUP_SUBPROOF_ID[0]),
            Some(LOOKUP_3_AIR_IDS),
        );

        lookup3
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Lookup".into(), LOOKUP_3_AIR_IDS[0])
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(
            LOOKUP_SUBPROOF_ID[0],
            LOOKUP_3_AIR_IDS[0],
            None,
            Some(buffer),
        );
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup3<F> {
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
                .get_buffer_info("Lookup".into(), LOOKUP_3_AIR_IDS[0])
                .unwrap();

            let buffer = air_instance.buffer.as_mut().unwrap();

            let num_rows = pctx
                .pilout
                .get_air(LOOKUP_SUBPROOF_ID[0], LOOKUP_3_AIR_IDS[0])
                .num_rows();
            let mut trace =
                Lookup33Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize)
                    .unwrap();

            for i in 0..num_rows {
                trace[i].c1 = F::from_canonical_usize(i);
                trace[i].d1 = F::from_canonical_usize(i);
                if i < 2usize.pow(12) {
                    trace[i].mul1 = F::from_canonical_usize(1);
                }

                trace[i].c2 = F::from_canonical_usize(i);
                trace[i].d2 = F::from_canonical_usize(i);
                if i < 2usize.pow(12) {
                    trace[i].mul2 = F::from_canonical_usize(1);
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
