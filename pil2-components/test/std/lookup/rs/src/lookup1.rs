use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use p3_field::PrimeField;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{Lookup11Trace, LOOKUP_1_AIR_IDS, LOOKUP_AIRGROUP_ID};

pub struct Lookup1<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Lookup1<F>
where
    Standard: Distribution<F>,
{
    const MY_NAME: &'static str = "Lookup1";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let lookup1 = Arc::new(Self { _phantom: std::marker::PhantomData });

        wcm.register_component(lookup1.clone(), Some(LOOKUP_AIRGROUP_ID), Some(LOOKUP_1_AIR_IDS));

        lookup1
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // For simplicity, add a single instance of each air
        let (buffer_size, _) =
            ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, LOOKUP_AIRGROUP_ID, LOOKUP_1_AIR_IDS[0]).unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(LOOKUP_AIRGROUP_ID, LOOKUP_1_AIR_IDS[0], None, buffer);

        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup1<F>
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
        let air = pctx.pilout.get_air(air_instance.airgroup_id, air_instance.air_id);

        log::info!(
            "{}: ··· Witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            air.name().unwrap_or("unknown"),
            stage
        );

        if stage == 1 {
            let (_buffer_size, offsets) =
                ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, LOOKUP_AIRGROUP_ID, LOOKUP_1_AIR_IDS[0]).unwrap();

            let buffer = &mut air_instance.buffer;

            let num_rows = pctx.pilout.get_air(LOOKUP_AIRGROUP_ID, LOOKUP_1_AIR_IDS[0]).num_rows();
            let mut trace = Lookup11Trace::map_buffer(buffer.as_mut_slice(), num_rows, offsets[0] as usize).unwrap();

            let num_lookups = trace[0].sel.len();

            for i in 0..num_rows {
                let val = rng.gen();
                let mut n_sel = 0;
                for j in 0..num_lookups {
                    trace[i].f[j] = val;
                    let selected = rng.gen_bool(0.5);
                    trace[i].sel[j] = F::from_bool(selected);
                    if selected {
                        n_sel += 1;
                    }
                }
                trace[i].t = val;
                trace[i].mul = F::from_canonical_usize(n_sel);
            }
        }
    }
}
