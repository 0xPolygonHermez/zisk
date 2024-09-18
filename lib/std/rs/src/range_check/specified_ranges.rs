use core::panic;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{
    AirInstance, AirInstancesRepository, ExecutionCtx, ProofCtx, SetupCtx, SetupRepository,
};
use proofman_hints::{
    get_hint_field, get_hint_field_constant, get_hint_ids_by_name, print_by_name, set_hint_field, HintFieldOptions, HintFieldValue
};

use crate::Range;

const PROVE_CHUNK_SIZE: usize = 1 << 5;

pub struct SpecifiedRanges<F: PrimeField> {
    // Proof-related data
    setup_repository: RefCell<Arc<SetupRepository>>,
    public_inputs: Arc<RefCell<Vec<u8>>>,
    challenges: Arc<RefCell<Vec<F>>>,
    air_instances_repository: RefCell<Arc<AirInstancesRepository<F>>>,
    // Parameters
    hints: RefCell<Vec<u64>>,
    airgroup_id: usize,
    air_id: usize,
    // Inputs
    num_rows: RefCell<usize>,
    ranges: Mutex<Vec<Range<F>>>,
    inputs: Mutex<Vec<(Range<F>, F)>>, // range -> value -> multiplicity
    muls: RefCell<Vec<HintFieldValue<F>>>,
}

impl<F: PrimeField> SpecifiedRanges<F> {
    const MY_NAME: &'static str = "SpecifiedRanges";

    pub fn new(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let specified_ranges = Arc::new(Self {
            setup_repository: RefCell::new(Arc::new(SetupRepository { setups: Vec::new() })),
            public_inputs: Arc::new(RefCell::new(Vec::new())),
            challenges: Arc::new(RefCell::new(Vec::new())),
            air_instances_repository: RefCell::new(Arc::new(AirInstancesRepository::new())),
            hints: RefCell::new(Vec::new()),
            airgroup_id,
            air_id,
            num_rows: RefCell::new(0),
            ranges: Mutex::new(Vec::new()),
            inputs: Mutex::new(Vec::new()),
            muls: RefCell::new(Vec::new()),
        });

        wcm.register_component(specified_ranges.clone(), Some(airgroup_id), Some(&[air_id]));

        specified_ranges
    }

    pub fn update_inputs(&self, value: F, range: Range<F>) {
        let mut inputs = self.inputs.lock().unwrap();
        inputs.push((range, value));

        while inputs.len() >= PROVE_CHUNK_SIZE {
            let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
            let drained_inputs = inputs.drain(..num_drained).collect();

            // Update the multiplicity column
            self.update_multiplicity(drained_inputs);

            log::info!(
                "{}: Updated inputs for AIR '{}'",
                Self::MY_NAME,
                "SpecifiedRanges"
            );
        }
    }

    pub fn drain_inputs(&self) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect();

        // Perform the last update
        self.update_multiplicity(drained_inputs);

        log::info!(
            "{}: Drained inputs for AIR '{}'",
            Self::MY_NAME,
            "SpecifiedRanges"
        );
    }

    fn update_multiplicity(&self, drained_inputs: Vec<(Range<F>, F)>) {
        // TODO! Do it in parallel
        let ranges = self.ranges.lock().unwrap();
        let num_rows = self.num_rows.borrow().clone();
        for (range, input) in &drained_inputs {
            let value = *input - range.0;

            let value = value
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
            self.muls.borrow_mut()[range_index].add(index, F::one());
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for SpecifiedRanges<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        // TODO: We can optimize this
        // Scan the pilout for airs that have rc-related hints
        let air_groups = pctx.pilout.air_groups();
        for air_group in air_groups.iter() {
            let airs = air_group.airs();
            for air in airs.iter() {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.setups.get_setup(airgroup_id, air_id).expect("REASON");

                let hints = get_hint_ids_by_name(setup.p_setup, "specified_ranges");
                for (index, hint) in hints.iter().enumerate() {
                    if index > 0 {
                        let min = get_hint_field_constant::<F>(
                            sctx,
                            airgroup_id,
                            air_id,
                            *hint as usize,
                            "min",
                            HintFieldOptions::default(),
                        );
                        let max = get_hint_field_constant::<F>(
                            sctx,
                            airgroup_id,
                            air_id,
                            *hint as usize,
                            "max",
                            HintFieldOptions::default(),
                        );
                        let HintFieldValue::Field(min) = min else {
                            log::error!("Min hint must be a field element");
                            panic!();
                        };
                        let HintFieldValue::Field(max) = max else {
                            log::error!("Min_neg hint must be a field element");
                            panic!();
                        };
                        self.ranges
                            .lock()
                            .unwrap()
                            .push(Range(min, max, false, false));
                    }

                    self.hints.borrow_mut().push(*hint);
                }
            }
        }

        self.setup_repository.replace(sctx.setups.clone());
        self.air_instances_repository
            .replace(pctx.air_instance_repo.clone());

        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("SpecifiedRanges".into(), self.air_id)
            .unwrap();
        let buffer = vec![F::zero(); buffer_size as usize];

        // Add a new air instance. Since Specified Ranges is a table, only this air instance is needed
        let mut air_instance = AirInstance::new(self.airgroup_id, self.air_id, None, buffer);
        for hint in self.hints.borrow().iter().skip(1) {
            self.muls.borrow_mut().push(get_hint_field::<F>(
                self.setup_repository.borrow().as_ref(),
                self.public_inputs.clone(),
                self.challenges.clone(),
                &mut air_instance,
                hint.to_usize().unwrap(),
                "reference",
                HintFieldOptions::dest(),
            ));
        }

        // Set the number of rows
        let hint = self.hints.borrow()[0];

        let num_rows = get_hint_field::<F>(
            self.setup_repository.borrow().as_ref(),
            self.public_inputs.clone(),
            self.challenges.clone(),
            &mut air_instance,
            hint as usize,
            "num_rows",
            HintFieldOptions::dest(),
        );

        let HintFieldValue::Field(num_rows) = num_rows else {
            log::error!("Number of rows must be a field element");
            panic!();
        };

        self.num_rows
            .replace(num_rows.as_canonical_biguint().to_usize().unwrap());

        self.air_instances_repository
            .borrow_mut()
            .add_air_instance(air_instance);
    }

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
        // Set the multiplicity columns as done
        let hints = self.hints.borrow();

        let air_instance_id = self
            .air_instances_repository
            .borrow()
            .find_air_instances(self.airgroup_id, self.air_id)[0];

        let air_instances = self.air_instances_repository.borrow();
        let mut air_instance_rw = air_instances.air_instances.write().unwrap();
        let air_instance = &mut air_instance_rw[air_instance_id];

        let mul = self.muls.borrow();
        for (index, hint) in hints.iter().enumerate().skip(1) {
            set_hint_field(
                self.setup_repository.borrow().as_ref(),
                air_instance,
                *hint,
                "reference",
                &mul[index - 1],
            );
            let index = index - 1;
            print_by_name(
                _sctx,
                _pctx,
                air_instance,
                "SpecifiedRanges.mul",
                Some(vec![index as u64]),
                0,
                2,
            );
        }
    }
}
