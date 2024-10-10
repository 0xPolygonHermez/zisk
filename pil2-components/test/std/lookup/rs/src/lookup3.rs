use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;

use crate::{Lookup35Trace, LOOKUP_3_AIR_IDS, LOOKUP_AIRGROUP_ID};

pub struct Lookup3<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Lookup3<F> {
    const MY_NAME: &'static str = "Lookup_3";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let lookup3 = Arc::new(Self { _phantom: std::marker::PhantomData });

        wcm.register_component(lookup3.clone(), Some(LOOKUP_AIRGROUP_ID), Some(LOOKUP_3_AIR_IDS));

        lookup3
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) =
            ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, LOOKUP_AIRGROUP_ID, LOOKUP_3_AIR_IDS[0]).unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(LOOKUP_AIRGROUP_ID, LOOKUP_3_AIR_IDS[0], None, buffer);

        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup3<F> {
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
        let air = pctx.pilout.get_air(air_instance.airgroup_id, air_instance.air_id);

        log::debug!(
            "{}: ··· Witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );

        if stage == 1 {
            let (_, offsets) =
                ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, LOOKUP_AIRGROUP_ID, LOOKUP_3_AIR_IDS[0]).unwrap();

            let buffer = &mut air_instance.buffer;

            let num_rows = pctx.pilout.get_air(LOOKUP_AIRGROUP_ID, LOOKUP_3_AIR_IDS[0]).num_rows();
            let mut trace = Lookup35Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize).unwrap();

            for i in 0..num_rows {
                trace[i].c1 = F::from_canonical_usize(i);
                trace[i].d1 = F::from_canonical_usize(i);
                if i < (1 << 12) {
                    trace[i].mul1 = F::from_canonical_usize(4);
                } else if i < (1 << 13) {
                    trace[i].mul1 = F::from_canonical_usize(3);
                } else {
                    trace[i].mul1 = F::from_canonical_usize(2);
                }

                trace[i].c2 = F::from_canonical_usize(i);
                trace[i].d2 = F::from_canonical_usize(i);
                if i < (1 << 12) {
                    trace[i].mul2 = F::from_canonical_usize(4);
                } else if i < (1 << 13) {
                    trace[i].mul2 = F::from_canonical_usize(3);
                } else {
                    trace[i].mul2 = F::from_canonical_usize(2);
                }
            }
        }
    }
}
