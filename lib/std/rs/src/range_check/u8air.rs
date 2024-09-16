use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{
    AirInstance, ExecutionCtx, ProofCtx, Setup, SetupCtx,
};

use proofman_common as common;
pub use proofman_macros::trace;

// PIL Helpers
trace!(U8Air0Row, U8Air0Trace<F> {
    mul: F,
});

const PROVE_CHUNK_SIZE: usize = 1 << 10;

pub struct U8Air<F> {
    airgroup_id: usize,
    air_id: usize,
    inputs: Mutex<Vec<F>>, // value -> multiplicity
    u8air_table: Mutex<Vec<F>>,
    offset: Mutex<usize>,

    // Setup context reference
    sctx: RefCell<Arc<Vec<Setup>>>,
}

impl<F: PrimeField> U8Air<F> {
    const MY_NAME: &'static str = "U8Air";

    pub fn new(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let u8air = Arc::new(Self {
            airgroup_id,
            air_id,
            inputs: Mutex::new(Vec::new()),
            u8air_table: Mutex::new(Vec::new()),
            offset: Mutex::new(0),
            sctx: RefCell::new(Arc::new(Vec::new())),
        });

        wcm.register_component(u8air.clone(), Some(airgroup_id), Some(&[air_id]));

        u8air
    }

    pub fn drain_inputs(&self) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect::<Vec<_>>();

        self.update_multiplicity(drained_inputs);

        println!("{}: Drained inputs for AIR 'U8Air'", Self::MY_NAME);
    }

    pub fn update_inputs(&self, value: F) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.push(value);

            while inputs.len() >= PROVE_CHUNK_SIZE {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                self.update_multiplicity(drained_inputs);
            }
        }
    }

    fn update_multiplicity(&self, drained_inputs: Vec<F>) {
        // TODO! Do it in parallel
        // Update the multiplicity column
        let num_rows = 1 << 8;
        let mut u8air_table = self.u8air_table.lock().unwrap();
        let offset = *self.offset.lock().unwrap();
        let mut trace = U8Air0Trace::map_buffer(&mut u8air_table, num_rows, offset).unwrap();

        for input in &drained_inputs {
            let value = input
                .as_canonical_biguint()
                .to_usize()
                .expect("Cannot convert to usize");
            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            trace[value % num_rows].mul += F::one();
        }

        log::info!("{}: Updated inputs for AIR '{}'", Self::MY_NAME, "U8Air");
    }
}

impl<F: PrimeField> WitnessComponent<F> for U8Air<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.sctx.replace(sctx.setups.clone());

        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("U8Air".into(), self.air_id)
            .unwrap();
        let buffer = vec![F::zero(); buffer_size as usize];

        let air_instance = AirInstance::new(self.airgroup_id, self.air_id, None, buffer);
        pctx.air_instance_repo.add_air_instance(air_instance);
    }

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }
}
