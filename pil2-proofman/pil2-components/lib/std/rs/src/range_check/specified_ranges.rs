use core::panic;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use witness::WitnessComponent;
use proofman_common::{AirInstance, ProofCtx, SetupCtx, TraceInfo};
use proofman_hints::{
    get_hint_field, get_hint_field_constant, get_hint_ids_by_name, set_hint_field, HintFieldOptions, HintFieldValue,
};
use proofman_util::create_buffer_fast;

use crate::{AirComponent, Range};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct SpecifiedRanges<F: PrimeField> {
    // Parameters
    hints: Mutex<Vec<u64>>,
    airgroup_id: usize,
    air_id: usize,

    // Inputs
    num_rows: Mutex<usize>,
    ranges: Mutex<Vec<Range<F>>>,
    inputs: Mutex<Vec<(Range<F>, F, F)>>, // range -> value -> multiplicity
    mul_columns: Mutex<Vec<HintFieldValue<F>>>,
}

impl<F: PrimeField> AirComponent<F> for SpecifiedRanges<F> {
    const MY_NAME: &'static str = "SpecRang";

    fn new(
        _pctx: Arc<ProofCtx<F>>,
        _sctx: Arc<SetupCtx>,
        airgroup_id: Option<usize>,
        air_id: Option<usize>,
    ) -> Arc<Self> {
        let airgroup_id = airgroup_id.expect("Airgroup ID must be provided");
        let air_id = air_id.expect("Air ID must be provided");
        Arc::new(Self {
            hints: Mutex::new(Vec::new()),
            airgroup_id,
            air_id,
            num_rows: Mutex::new(0),
            ranges: Mutex::new(Vec::new()),
            inputs: Mutex::new(Vec::new()),
            mul_columns: Mutex::new(Vec::new()),
        })
    }
}

impl<F: PrimeField> SpecifiedRanges<F> {
    pub fn update_inputs(&self, value: F, range: Range<F>, multiplicity: F) {
        let mut inputs = self.inputs.lock().unwrap();
        inputs.push((range, value, multiplicity));

        while inputs.len() >= PROVE_CHUNK_SIZE {
            let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
            let drained_inputs = inputs.drain(..num_drained).collect();

            // Update the multiplicity column
            self.update_multiplicity(drained_inputs);
        }
    }

    pub fn drain_inputs(&self, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect();

        // Perform the last update
        self.update_multiplicity(drained_inputs);

        let mut multiplicities = self
            .mul_columns
            .lock()
            .unwrap()
            .iter()
            .map(|column| match column {
                HintFieldValue::Column(values) => {
                    values.iter().map(|x| x.as_canonical_biguint().to_u64().unwrap()).collect::<Vec<u64>>()
                }
                _ => panic!("Multiplicities must be columns"),
            })
            .collect::<Vec<Vec<u64>>>();

        let (instance_found, global_idx) = pctx.dctx_find_instance(self.airgroup_id, self.air_id);

        let (is_mine, global_idx) = if instance_found {
            (pctx.dctx_is_my_instance(global_idx), global_idx)
        } else {
            pctx.dctx_add_instance(self.airgroup_id, self.air_id, pctx.get_weight(self.airgroup_id, self.air_id))
        };

        pctx.dctx_distribute_multiplicities(&mut multiplicities, global_idx);

        if is_mine {
            // Set the multiplicity columns as done
            let hints = self.hints.lock().unwrap();

            let instance: Vec<usize> = pctx.air_instance_repo.find_air_instances(self.airgroup_id, self.air_id);
            if instance.is_empty() {
                let num_rows = pctx.global_info.airs[self.airgroup_id][self.air_id].num_rows;
                let buffer_size = multiplicities.len() * num_rows;
                let buffer: Vec<F> = create_buffer_fast(buffer_size);
                let air_instance = AirInstance::new(TraceInfo::new(self.airgroup_id, self.air_id, buffer));
                pctx.add_air_instance(air_instance, global_idx);
            };
            let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = air_instances.get_mut(&global_idx).unwrap();

            let mul_columns_2 = multiplicities
                .iter()
                .map(|multiplicities| {
                    HintFieldValue::Column(multiplicities.iter().map(|x| F::from_canonical_u64(*x)).collect::<Vec<F>>())
                })
                .collect::<Vec<HintFieldValue<F>>>();

            for (index, hint) in hints[1..].iter().enumerate() {
                set_hint_field(&sctx, air_instance, *hint, "reference", &mul_columns_2[index]);
            }

            log::trace!("{}: ··· Drained inputs for AIR '{}'", Self::MY_NAME, "SpecifiedRanges");
        }
    }

    fn update_multiplicity(&self, drained_inputs: Vec<(Range<F>, F, F)>) {
        // TODO! Do it in parallel
        let ranges = self.ranges.lock().unwrap();

        let num_rows = self.num_rows.lock().unwrap();
        let mut mul_columns = self.mul_columns.lock().unwrap();
        for (range, input, mul) in &drained_inputs {
            let value = *input - range.0;

            let value = value
                .as_canonical_biguint()
                .to_usize()
                .unwrap_or_else(|| panic!("Cannot convert to usize: {:?}", value));

            let range_index =
                ranges.iter().position(|r| r == range).unwrap_or_else(|| panic!("Range {:?} not found", range));

            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            let index = value % *num_rows;
            mul_columns[range_index].add(index, *mul);
        }
    }

    pub fn airgroup_id(&self) -> usize {
        self.airgroup_id
    }

    pub fn air_id(&self) -> usize {
        self.air_id
    }
}

impl<F: PrimeField> WitnessComponent<F> for SpecifiedRanges<F> {
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        // Obtain info from the mul hints
        let setup = sctx.get_setup(self.airgroup_id, self.air_id);
        let specified_hints = get_hint_ids_by_name(setup.p_setup.p_expressions_bin, "specified_ranges");
        let mut hints_guard = self.hints.lock().unwrap();
        let mut ranges_guard = self.ranges.lock().unwrap();
        if !specified_hints.is_empty() {
            for (index, hint) in specified_hints.iter().enumerate() {
                if index >= 1 {
                    let predefined = get_hint_field_constant::<F>(
                        &sctx,
                        self.airgroup_id,
                        self.air_id,
                        *hint as usize,
                        "predefined",
                        HintFieldOptions::default(),
                    );
                    let min = get_hint_field_constant::<F>(
                        &sctx,
                        self.airgroup_id,
                        self.air_id,
                        *hint as usize,
                        "min",
                        HintFieldOptions::default(),
                    );
                    let min_neg = get_hint_field_constant::<F>(
                        &sctx,
                        self.airgroup_id,
                        self.air_id,
                        *hint as usize,
                        "min_neg",
                        HintFieldOptions::default(),
                    );
                    let max = get_hint_field_constant::<F>(
                        &sctx,
                        self.airgroup_id,
                        self.air_id,
                        *hint as usize,
                        "max",
                        HintFieldOptions::default(),
                    );
                    let max_neg = get_hint_field_constant::<F>(
                        &sctx,
                        self.airgroup_id,
                        self.air_id,
                        *hint as usize,
                        "max_neg",
                        HintFieldOptions::default(),
                    );

                    let HintFieldValue::Field(predefined) = predefined else {
                        log::error!("Predefined hint must be a field element");
                        panic!();
                    };
                    let predefined = {
                        if !predefined.is_zero() && !predefined.is_one() {
                            log::error!("Predefined hint must be either 0 or 1");
                            panic!();
                        }
                        predefined.is_one()
                    };
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

                    ranges_guard.push(Range(min, max, min_neg, max_neg, predefined));
                }

                hints_guard.push(*hint);
            }
        }

        let num_rows = pctx.global_info.airs[self.airgroup_id][self.air_id].num_rows;
        let buffer_size = specified_hints.len() * num_rows;
        let buffer = create_buffer_fast(buffer_size as usize);

        // Add a new air instance. Since Specified Ranges is a table, only this air instance is needed
        let mut air_instance = AirInstance::new(TraceInfo::new(self.airgroup_id, self.air_id, buffer));
        let mut mul_columns_guard = self.mul_columns.lock().unwrap();
        for hint in hints_guard[1..].iter() {
            mul_columns_guard.push(get_hint_field::<F>(
                &sctx,
                &pctx,
                &mut air_instance,
                hint.to_usize().unwrap(),
                "reference",
                HintFieldOptions::dest_with_zeros(),
            ));
        }

        // Set the number of rows
        let hint = hints_guard[0];

        let num_rows = get_hint_field_constant::<F>(
            &sctx,
            self.airgroup_id,
            self.air_id,
            hint as usize,
            "num_rows",
            HintFieldOptions::default(),
        );

        let HintFieldValue::Field(num_rows) = num_rows else {
            log::error!("Number of rows must be a field element");
            panic!();
        };

        *self.num_rows.lock().unwrap() = num_rows.as_canonical_biguint().to_usize().unwrap();
    }

    fn calculate_witness(&self, stage: u32, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        if stage == 1 {
            Self::drain_inputs(self, pctx, sctx);
        }
    }
}
