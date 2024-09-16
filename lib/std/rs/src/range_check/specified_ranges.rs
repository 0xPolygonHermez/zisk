use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{
    AirInstance, AirInstancesRepository, ExecutionCtx, ProofCtx, SetupCtx, SetupRepository,
};
use proofman_hints::{get_hint_field, set_hint_field, HintFieldOutput, HintFieldValue};

use crate::Range;

const PROVE_CHUNK_SIZE: usize = 1 << 10;

pub struct SpecifiedRanges<F: PrimeField> {
    // Proof-related data
    setup_repository: RefCell<Arc<SetupRepository>>,
    public_inputs: Arc<Vec<u8>>,
    challenges: Arc<RefCell<Vec<F>>>,
    air_instances_repository: RefCell<Arc<AirInstancesRepository<F>>>,
    // Parameters
    pub hints: Mutex<Vec<u64>>,
    airgroup_id: usize,
    air_id: usize,
    // Inputs
    num_rows: usize,
    ranges: Mutex<Vec<Range<F>>>,
    inputs: Mutex<Vec<(Range<F>, F)>>, // range -> value -> multiplicity
}

impl<F: PrimeField> SpecifiedRanges<F> {
    const MY_NAME: &'static str = "SpecifiedRanges";

    pub fn new(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let specified_ranges = Arc::new(Self {
            setup_repository: RefCell::new(Arc::new(SetupRepository { setups: Vec::new() })),
            public_inputs: Arc::new(Vec::new()),
            challenges: Arc::new(RefCell::new(Vec::new())),
            air_instances_repository: RefCell::new(Arc::new(AirInstancesRepository::new())),
            hints: Mutex::new(Vec::new()),
            airgroup_id,
            air_id,
            num_rows: 0,
            ranges: Mutex::new(Vec::new()),
            inputs: Mutex::new(Vec::new()),
        });

        wcm.register_component(specified_ranges.clone(), Some(airgroup_id), Some(&[air_id]));

        specified_ranges
    }

    pub fn drain_inputs(&self) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect::<Vec<_>>();

        self.update_multiplicity(drained_inputs);

        // Set the multiplicity column as done
        let hint = self.hint.lock().unwrap();

        let air_instance_id = self
            .air_instances_repository
            .borrow()
            .find_air_instances(self.airgroup_id, self.air_id)[0];
        let mut air_instance_rw = self
            .air_instances_repository
            .borrow()
            .air_instances
            .write()
            .unwrap();
        let air_instance = &mut air_instance_rw[air_instance_id];

        let mut mul = get_hint_field::<F>(
            self.setup_repository.borrow().as_ref(),
            self.public_inputs.clone(),
            self.challenges.clone(),
            air_instance,
            *hint as usize,
            "reference",
            true,
            false,
            false,
        );

        set_hint_field(sctx, air_instance, *hint as u64, "reference", &mut mul);

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

        let air_instance_id = self
            .air_instances_repository
            .borrow()
            .find_air_instances(self.airgroup_id, self.air_id)[0];
        let air_instance_bind = self.air_instances_repository.borrow();
        let mut air_instance_rw = air_instance_bind.air_instances.write().unwrap();
        let air_instance = &mut air_instance_rw[air_instance_id];

        let mut mul = get_hint_field::<F>(
            self.setup_repository.borrow().as_ref(),
            self.public_inputs.clone(),
            self.challenges.clone(),
            air_instance,
            hint,
            "reference",
            true,
            false,
            false,
        );

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
            let index = value % num_rows;
            let previous_mul_val = mul.get(index);
            mul.set(index, previous_mul_val + HintFieldOutput::Field(F::one()));

            // trace[value % num_rows].mul[range_index] += F::one();
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
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.setup_repository.replace(sctx.setups.clone());
        self.air_instances_repository
            .replace(pctx.air_instance_repo.clone());

        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("U16Air".into(), self.air_id)
            .unwrap();
        let buffer = vec![F::zero(); buffer_size as usize];

        // Add a new air instance. Since Specified Ranges is a table, only this air instance is needed
        let mut air_instance = AirInstance::new(self.airgroup_id, self.air_id, None, buffer);
        self.air_instances_repository
            .borrow_mut()
            .add_air_instance(air_instance);

        // Set the number of rows
        let hint = self.hints.lock().unwrap()[0] as usize;

        let mut num_rows = get_hint_field::<F>(
            self.setup_repository.borrow().as_ref(),
            self.public_inputs.clone(),
            self.challenges.clone(),
            &mut air_instance,
            hint,
            "reference",
            true,
            false,
            false,
        );

        let HintFieldValue::Field(num_rows) = num_rows else {
            log::error!("Max_neg hint must be a field element");
            panic!();
        };

        self.num_rows = num_rows
            .as_canonical_biguint()
            .to_usize()
            .expect("Cannot convert to usize");
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
