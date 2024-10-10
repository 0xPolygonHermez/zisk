use core::panic;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{
    get_hint_field, get_hint_field_constant, get_hint_ids_by_name, set_hint_field, HintFieldOptions, HintFieldValue,
};
use proofman_util::create_buffer_fast;

use crate::Range;

const PROVE_CHUNK_SIZE: usize = 1 << 5;

pub struct SpecifiedRanges<F: PrimeField> {
    wcm: Arc<WitnessManager<F>>,

    // Parameters
    hints: Mutex<Vec<u64>>,
    airgroup_id: usize,
    air_id: usize,

    // Inputs
    num_rows: Mutex<usize>,
    ranges: Mutex<Vec<Range<F>>>,
    inputs: Mutex<Vec<(Range<F>, F)>>, // range -> value -> multiplicity
    muls: Mutex<Vec<HintFieldValue<F>>>,
}

impl<F: PrimeField> SpecifiedRanges<F> {
    const MY_NAME: &'static str = "SpecRang";

    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let specified_ranges = Arc::new(Self {
            wcm: wcm.clone(),
            hints: Mutex::new(Vec::new()),
            airgroup_id,
            air_id,
            num_rows: Mutex::new(0),
            ranges: Mutex::new(Vec::new()),
            inputs: Mutex::new(Vec::new()),
            muls: Mutex::new(Vec::new()),
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
        }
    }

    pub fn drain_inputs(&self) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect();

        // Perform the last update
        self.update_multiplicity(drained_inputs);

        // Set the multiplicity columns as done
        let hints = self.hints.lock().unwrap();

        let air_instance_repo = &self.wcm.get_pctx().air_instance_repo;
        let air_instance_id = air_instance_repo.find_air_instances(self.airgroup_id, self.air_id)[0];
        let mut air_instance_rw = air_instance_repo.air_instances.write().unwrap();
        let air_instance = &mut air_instance_rw[air_instance_id];

        let mul = &*self.muls.lock().unwrap();

        for (index, hint) in hints.iter().enumerate().skip(1) {
            set_hint_field(self.wcm.get_sctx(), air_instance, *hint, "reference", &mul[index - 1]);
        }

        log::trace!("{}: ··· Drained inputs for AIR '{}'", Self::MY_NAME, "SpecifiedRanges");
    }

    fn update_multiplicity(&self, drained_inputs: Vec<(Range<F>, F)>) {
        // TODO! Do it in parallel
        let ranges = self.ranges.lock().unwrap();

        let num_rows = self.num_rows.lock().unwrap();
        let mut muls = self.muls.lock().unwrap();
        for (range, input) in &drained_inputs {
            let value = *input - range.0;

            let value = value.as_canonical_biguint().to_usize().expect("Cannot convert to usize");

            let range_index = ranges.iter().position(|r| r == range).expect("Range not found");

            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            let index = value % *num_rows;
            muls[range_index].add(index, F::one());
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for SpecifiedRanges<F> {
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // TODO: We can optimize this
        // Scan the pilout for airs that have rc-related hints
        let air_groups = pctx.pilout.air_groups();
        let mut hints_guard = self.hints.lock().unwrap();

        let mut ranges_guard = self.ranges.lock().unwrap();

        for air_group in air_groups.iter() {
            let airs = air_group.airs();
            for air in airs.iter() {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;

                let setup = sctx.get_partial_setup(airgroup_id, air_id).expect("REASON");
                let hints = get_hint_ids_by_name(setup.p_setup.p_expressions_bin, "specified_ranges");

                for (index, hint) in hints.iter().enumerate() {
                    if index > 0 {
                        let min = get_hint_field_constant::<F>(
                            &sctx,
                            airgroup_id,
                            air_id,
                            *hint as usize,
                            "min",
                            HintFieldOptions::default(),
                        );
                        let min_neg = get_hint_field_constant::<F>(
                            &sctx,
                            airgroup_id,
                            air_id,
                            *hint as usize,
                            "min_neg",
                            HintFieldOptions::default(),
                        );
                        let max = get_hint_field_constant::<F>(
                            &sctx,
                            airgroup_id,
                            air_id,
                            *hint as usize,
                            "max",
                            HintFieldOptions::default(),
                        );
                        let max_neg = get_hint_field_constant::<F>(
                            &sctx,
                            airgroup_id,
                            air_id,
                            *hint as usize,
                            "max_neg",
                            HintFieldOptions::default(),
                        );
                        let HintFieldValue::Field(min) = min else {
                            log::error!("Min hint must be a field element");
                            panic!();
                        };
                        let min_neg = match min_neg {
                            HintFieldValue::Field(value) => {
                                if value.is_zero() {
                                    false
                                } else if value.is_one() {
                                    true
                                } else {
                                    log::error!("Predefined hint must be either 0 or 1");
                                    panic!("Invalid predefined hint value"); // Or return Err if you prefer error handling
                                }
                            }
                            _ => {
                                log::error!("Max_neg hint must be a field element");
                                panic!("Invalid hint type"); // Or return Err if you prefer error handling
                            }
                        };
                        let HintFieldValue::Field(max) = max else {
                            log::error!("Max hint must be a field element");
                            panic!();
                        };
                        let max_neg = match max_neg {
                            HintFieldValue::Field(value) => {
                                if value.is_zero() {
                                    false
                                } else if value.is_one() {
                                    true
                                } else {
                                    log::error!("Predefined hint must be either 0 or 1");
                                    panic!("Invalid predefined hint value"); // Or return Err if you prefer error handling
                                }
                            }
                            _ => {
                                log::error!("Max_neg hint must be a field element");
                                panic!("Invalid hint type"); // Or return Err if you prefer error handling
                            }
                        };

                        ranges_guard.push(Range(min, max, min_neg, max_neg));
                    }

                    hints_guard.push(*hint);
                }
            }
        }

        let (buffer_size, _) =
            ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, self.airgroup_id, self.air_id).unwrap();
        let buffer = create_buffer_fast(buffer_size as usize);

        // Add a new air instance. Since Specified Ranges is a table, only this air instance is needed
        let mut air_instance = AirInstance::new(self.airgroup_id, self.air_id, None, buffer);

        let mut muls_guard = self.muls.lock().unwrap();

        for hint in hints_guard.iter().skip(1) {
            muls_guard.push(get_hint_field::<F>(
                &sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                &mut air_instance,
                hint.to_usize().unwrap(),
                "reference",
                HintFieldOptions::dest(),
            ));
        }

        // Set the number of rows
        let hint = hints_guard[0];

        let num_rows = get_hint_field::<F>(
            &sctx,
            &pctx.public_inputs,
            &pctx.challenges,
            &mut air_instance,
            hint as usize,
            "num_rows",
            HintFieldOptions::dest(),
        );

        let HintFieldValue::Field(num_rows) = num_rows else {
            log::error!("Number of rows must be a field element");
            panic!();
        };

        *self.num_rows.lock().unwrap() = num_rows.as_canonical_biguint().to_usize().unwrap();

        pctx.air_instance_repo.add_air_instance(air_instance);
    }

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}
