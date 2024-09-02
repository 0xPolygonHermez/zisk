use std::sync::Arc;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use pil_std_lib::Std;

use p3_field::PrimeField;
use rand::Rng;
use num_bigint::BigInt;

use crate::{Lookup20Trace, LOOKUP_SUBPROOF_ID, LOOKUP_2_AIR_IDS};

pub struct Lookup<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Copy> Lookup<F> {
    const MY_NAME: &'static str = "Lookup";

    pub fn new(wcm: &mut WitnessManager<F>) -> Arc<Self> {
        let lookup = Arc::new(Self {
            _phantom: std::marker::PhantomData,
        });

        wcm.register_component(lookup.clone(), Some(LOOKUP_SUBPROOF_ID));

        lookup
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // For simplicity, add a single instance of the air
        let (buffer_size, offsets) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Lookup".into(), LOOKUP_2_AIR_IDS[0])
            .unwrap();

        let mut buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(
            LOOKUP_SUBPROOF_ID[0],
            LOOKUP_2_AIR_IDS[0],
            Some(buffer),
        );
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for Lookup<F> {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance_id: Option<usize>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        // let mut rng = rand::thread_rng();

        log::info!(
            "{}: Initiating witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            "Lookup2",
            stage
        );

        let (buffer_size, offsets) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("Lookup".into(), LOOKUP_2_AIR_IDS[0])
            .unwrap();

        let mut buffer = vec![F::zero(); buffer_size as usize];

        let num_rows = pctx.pilout.get_air(LOOKUP_SUBPROOF_ID[0], LOOKUP_2_AIR_IDS[0]).num_rows();
        let mut trace = Lookup20Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize).unwrap();

        for i in 0..num_rows-1 {
            trace[i].a1 = F::from_canonical_usize(i);
            trace[i].b1 = F::from_canonical_usize(i+1);
            trace[i].c1 = F::from_canonical_usize(i);
            trace[i].d1 = F::from_canonical_usize(i+1);
        }
        trace[num_rows-1].a1 = F::from_canonical_usize(10);
        trace[num_rows-1].b1 = F::from_canonical_usize(10);
        trace[num_rows-1].c1 = F::from_canonical_usize(10);
        trace[num_rows-1].d1 = F::from_canonical_usize(11);
    }
}
