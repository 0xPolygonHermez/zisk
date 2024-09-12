use std::mem;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use proofman_common as common;
pub use proofman_macros::trace;
use rayon::Scope;

// PIL Helpers
trace!(U16Air0Row, U16Air0Trace<F> {
    mul: F,
});

const PROVE_CHUNK_SIZE: usize = 1 << 10;

pub struct U16Air<F> {
    airgroup_id: usize,
    air_id: usize,
    inputs: Mutex<Vec<F>>, // value -> multiplicity
    u16air_table: Mutex<Vec<F>>,
    offset: Mutex<usize>,
}

impl<F: PrimeField> U16Air<F> {
    const MY_NAME: &'static str = "U16Air";

    pub fn new(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let u16air = Arc::new(Self {
            airgroup_id,
            air_id,
            inputs: Mutex::new(Vec::new()),
            u16air_table: Mutex::new(Vec::new()),
            offset: Mutex::new(0),
        });

        wcm.register_component(u16air.clone(), Some(airgroup_id), Some(&[air_id]));

        u16air
    }

    pub fn drain_inputs(&self, pctx: &mut ProofCtx<F>, _scope: Option<&Scope>) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect::<Vec<_>>();

        self.update_multiplicity(drained_inputs);

        let u16air_table = mem::take(&mut *self.u16air_table.lock().unwrap());
        pctx.add_air_instance_ctx(self.airgroup_id, self.air_id, None, Some(u16air_table));

        println!("{}: Drained inputs for AIR 'U16Air'", Self::MY_NAME);
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
        let num_rows = 1 << 16;
        let mut u16air_table = self.u16air_table.lock().unwrap();
        let offset = *self.offset.lock().unwrap();
        let mut trace = U16Air0Trace::map_buffer(&mut u16air_table, num_rows, offset).unwrap();

        for input in &drained_inputs {
            let value = input
                .as_canonical_biguint()
                .to_usize()
                .expect("Cannot convert to usize");
            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            trace[value % num_rows].mul += F::one();
        }

        log::info!("{}: Updated inputs for AIR '{}'", Self::MY_NAME, "U16Air");
    }
}

impl<F: PrimeField> WitnessComponent<F> for U16Air<F> {
    fn start_proof(&self, _pctx: &ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        let (buffer_size, offsets) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("U16Air".into(), self.air_id)
            .unwrap();

        let mut u16air_table = self.u16air_table.lock().unwrap();
        *u16air_table = vec![F::zero(); buffer_size as usize];

        *self.offset.lock().unwrap() = offsets[0] as usize;
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
