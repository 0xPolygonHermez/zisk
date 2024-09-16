use std::mem;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use proofman_common as common;
pub use proofman_macros::trace;
use rayon::Scope;

use crate::Range;

// PIL Helpers
trace!(SpecifiedRanges0Row, SpecifiedRanges0Trace<F> {
    mul: [F; 1], // TODO: This number cannot be hardcorded, it depens on the air that instantiates the range check
});

const PROVE_CHUNK_SIZE: usize = 1 << 10;

pub struct SpecifiedRanges<F: PrimeField> {
    airgroup_id: usize,
    air_id: usize,
    ranges: Mutex<Vec<Range<F>>>,
    inputs: Mutex<Vec<(Range<F>, F)>>, // range -> value -> multiplicity
    specified_ranges_table: Mutex<Vec<F>>,
    offset: Mutex<usize>,
}

impl<F: PrimeField> SpecifiedRanges<F> {
    const MY_NAME: &'static str = "SpecifiedRanges";

    pub fn new(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let specified_ranges = Arc::new(Self {
            airgroup_id,
            air_id,
            ranges: Mutex::new(Vec::new()),
            inputs: Mutex::new(Vec::new()),
            specified_ranges_table: Mutex::new(Vec::new()),
            offset: Mutex::new(0),
        });

        wcm.register_component(specified_ranges.clone(), Some(airgroup_id), Some(&[air_id]));

        specified_ranges
    }

    pub fn drain_inputs(&self, pctx: &mut ProofCtx<F>, _scope: Option<&Scope>) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect::<Vec<_>>();

        self.update_multiplicity(drained_inputs);

        let specified_ranges_table = mem::take(&mut *self.specified_ranges_table.lock().unwrap());
        let air_instance =
            AirInstance::new(self.airgroup_id, self.air_id, None, specified_ranges_table);
        pctx.air_instance_repo.add_air_instance(air_instance);

        println!(
            "{}: Drained inputs for AIR 'Specified Ranges'",
            Self::MY_NAME
        );
    }

    pub fn update_inputs(&self, value: F, range: Range<F>) {
        if let Ok(mut inputs) = self.inputs.lock() {
            // Note: The order in the following vector is important for the multiplicity column
            if let Ok(mut ranges) = self.ranges.lock() {
                if !ranges.contains(&range) {
                    ranges.push(range);
                }
            }

            inputs.push((range, value));

            while inputs.len() >= PROVE_CHUNK_SIZE {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                self.update_multiplicity(drained_inputs);
            }
        }
    }

    fn update_multiplicity(&self, drained_inputs: Vec<(Range<F>, F)>) {
        // TODO! Do it in parallel
        // Update the multiplicity column
        let num_rows = 1 << 32; // TODO: Compute from ranges!!!
        let mut specified_ranges_table = self.specified_ranges_table.lock().unwrap();
        let offset = *self.offset.lock().unwrap();
        let mut trace =
            SpecifiedRanges0Trace::map_buffer(&mut specified_ranges_table, num_rows, offset)
                .unwrap();

        let ranges = self.ranges.lock().unwrap();
        for (range, input) in &drained_inputs {
            let value = input
                .as_canonical_biguint()
                .to_usize()
                .expect("Cannot convert to usize");

            let range_index = ranges
                .iter()
                .position(|r| r == range)
                .expect("Range not found");

            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            trace[value % num_rows].mul[range_index] += F::one();
        }

        // for k in 0..trace[0].mul.len() {
        //     let range = inputs
        //         .keys()
        //         .nth(k)
        //         .expect("Rc::calculate_trace() range not found");
        //     let min = range.0;
        //     let max = range.1;
        //     for i in 0..num_rows {
        //         // Ranges doesn't necessarily have to be a power of two
        //         // so we must adjust the multiplicity to that case
        //         if F::from_canonical_usize(i) >= max - min + F::one() {
        //             trace[k].mul[i] = F::zero();
        //         } else {
        //             trace[k].mul[i] = *inputs
        //                 .get(range)
        //                 .unwrap()
        //                 .clone()
        //                 .entry(F::from_canonical_usize(i))
        //                 .or_insert(F::zero());
        //         }
        //     }
        // }

        log::info!(
            "{}: Updated inputs for AIR '{}'",
            Self::MY_NAME,
            "SpecifiedRanges"
        );
    }
}

impl<F: PrimeField> WitnessComponent<F> for SpecifiedRanges<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("SpecifiedRanges".into(), self.air_id)
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
