use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    sync::{Arc, Mutex},
};

use num_bigint::BigInt;
use p3_field::PrimeField;

use proofman_common::{trace, AirInstanceCtx, ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{get_hint_field_constant, get_hint_ids_by_name, HintFieldValue};

use crate::Decider;

// PIL Helpers for the possible range check airs
trace!(U8Air0Row, U8Air0Trace<F> {
    mul: F,
});

trace!(U16Air0Row, U16Air0Trace<F> {
    mul: F,
});

trace!(SpecifiedRanges0Row, SpecifiedRanges0Trace<F> {
    mul: [F; 32], // TODO: This number cannot be hardcorded, it depens on the air that instantiates the range check
});

// pub enum RangeCheckTrace<F: 'static> {
//     U8Air0(U8Air0Trace<'static, F>),
//     U16Air0(U16Air0Trace<'static, F>),
//     SpecifiedRanges0(SpecifiedRanges0Trace<'static, F>),
// }

const BYTE: u8 = 255;
const TWOBYTES: u16 = 65535;

const STD_RANGE_CHECK_VARIANTS: usize = 3;
const STD_RANGE_CHECK_AIR_NAMES: [&str; STD_RANGE_CHECK_VARIANTS] =
    ["U8Air", "U16Air", "SpecifiedRanges"];

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum RangeCheckAir {
    U8Air,
    U16Air,
    SpecifiedRanges,
}

impl Display for RangeCheckAir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RangeCheckAir::U8Air => write!(f, "{}", STD_RANGE_CHECK_AIR_NAMES[0]),
            RangeCheckAir::U16Air => write!(f, "{}", STD_RANGE_CHECK_AIR_NAMES[1]),
            RangeCheckAir::SpecifiedRanges => write!(f, "{}", STD_RANGE_CHECK_AIR_NAMES[2]),
        }
    }
}

#[derive(Debug)]
pub struct RCAirData {
    pub air_name: RangeCheckAir,
    pub air_group_id: usize,
    pub air_id: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum StdRangeCheckType {
    Valid(RangeCheckAir),
    U8AirDouble,
    U16AirDouble,
}

type Range<F> = (F, F); // (min, max)

#[derive(Clone, Debug)]
struct StdRangeItem<F> {
    rc_type: StdRangeCheckType,
    range: Range<F>,
}

pub struct StdRangeCheck<F> {
    air_data: Option<Vec<RCAirData>>,
    ranges: Mutex<Vec<StdRangeItem<F>>>,
    inputs: Mutex<[HashMap<F, F>; STD_RANGE_CHECK_VARIANTS - 1]>, // value -> multiplicity
    inputs_specified: Mutex<HashMap<Range<F>, HashMap<F, F>>>,    // range -> value -> multiplicity
}

impl<F: PrimeField> Decider<F> for StdRangeCheck<F> {
    fn decide(
        &self,
        sctx: &SetupCtx,
        pctx: &ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        // Scan the pilout for airs that have rc-related hints
        let air_groups = pctx.pilout.air_groups();
        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let air_group_id = air.air_group_id;
                let air_id = air.air_id;
                let setup = sctx.get_setup(air_group_id, air_id).expect("REASON");
                let rc_hints = get_hint_ids_by_name(setup.p_expressions, "range_check");
                if !rc_hints.is_empty() {
                    // Register the ranges for the range check
                    self.register_ranges(sctx, air_group_id, air_id, rc_hints);
                }
            });
        });

        Ok(0)
    }
}

impl<F: PrimeField> StdRangeCheck<F> {
    const MY_NAME: &'static str = "STD Range Check";

    pub fn new(air_data: Option<Vec<RCAirData>>) -> Arc<Self> {
        // Check that the provided air data is valid
        if let Some(air_data) = air_data.as_ref() {
            if air_data.len() > STD_RANGE_CHECK_VARIANTS {
                log::error!(
                    "The air_data provided has incorrect lenght: expected at most {}, found {}",
                    STD_RANGE_CHECK_VARIANTS,
                    air_data.len()
                );
                panic!();
            }
        }

        Arc::new(Self {
            air_data,
            ranges: Mutex::new(Vec::new()),
            inputs: Mutex::new(core::array::from_fn(|_| HashMap::new())),
            inputs_specified: Mutex::new(HashMap::new()),
        })
    }

    // TODO!!!
    // pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
    //     // For simplicity, add a single instance of each air
    //     let (buffer_size, _) = ectx
    //         .buffer_allocator
    //         .as_ref()
    //         .get_buffer_info("Lookup".into(), LOOKUP_2_AIR_IDS[0])
    //         .unwrap();

    //     let buffer = vec![F::zero(); buffer_size as usize];

    //     pctx.add_air_instance_ctx(
    //         LOOKUP_SUBPROOF_ID[0],
    //         LOOKUP_2_AIR_IDS[0],
    //         Some(buffer),
    //     );
    // }

    pub fn register_ranges(
        &self,
        sctx: &SetupCtx,
        air_group_id: usize,
        air_id: usize,
        rc_hints: Vec<u64>,
    ) {
        for hint in rc_hints {
            let predefined = get_hint_field_constant::<F>(
                sctx,
                air_group_id,
                air_id,
                hint as usize,
                "predefined",
                false,
                false,
            );
            let min = get_hint_field_constant::<F>(
                sctx,
                air_group_id,
                air_id,
                hint as usize,
                "min",
                false,
                false,
            );
            let min_neg = get_hint_field_constant::<F>(
                sctx,
                air_group_id,
                air_id,
                hint as usize,
                "min_neg",
                false,
                false,
            );
            let max = get_hint_field_constant::<F>(
                sctx,
                air_group_id,
                air_id,
                hint as usize,
                "max",
                false,
                false,
            );
            let max_neg = get_hint_field_constant::<F>(
                sctx,
                air_group_id,
                air_id,
                hint as usize,
                "max_neg",
                false,
                false,
            );

            let HintFieldValue::Field(predefined) = predefined else {
                log::error!("Predefined hint must be a field element");
                panic!();
            };
            let HintFieldValue::Field(min) = min else {
                log::error!("Min hint must be a field element");
                panic!();
            };
            let HintFieldValue::Field(min_neg) = min_neg else {
                log::error!("Min_neg hint must be a field element");
                panic!();
            };
            let HintFieldValue::Field(max) = max else {
                log::error!("Max hint must be a field element");
                panic!();
            };
            let HintFieldValue::Field(max_neg) = max_neg else {
                log::error!("Max_neg hint must be a field element");
                panic!();
            };

            let predefined = {
                if !predefined.is_zero() && !predefined.is_one() {
                    log::error!("Predefined hint must be either 0 or 1");
                    panic!();
                }
                predefined.is_one()
            };
            let min_neg = {
                if !min_neg.is_zero() && !min_neg.is_one() {
                    log::error!("Predefined hint must be either 0 or 1");
                    panic!();
                }
                min_neg.is_one()
            };
            let max_neg = {
                if !max_neg.is_zero() && !max_neg.is_one() {
                    log::error!("Predefined hint must be either 0 or 1");
                    panic!();
                }
                max_neg.is_one()
            };

            // TODO: Associate this to a type!!!
            // // Convert min and max to BigInt
            // let mut min = BigInt::from(min.as_canonical_biguint());
            // let max = BigInt::from(max.as_canonical_biguint());

            // // Hint fields can only be expressed as field elements but in PIL they can be negative
            // // e.g.: on input range [-3,3], we obtain the range [p-3,3] which is counterintuitive
            // // we should therefore adjust the range to [-3,3]
            // if min > max {
            //     min -= BigInt::from(F::order());
            // }
            // // Note: It is impossible to distinguish between [-3,-2] and [p-3,p-2] (from a bigint perspective)
            // //       and, in fact, the range will be saved as [p-3,p-2]. However, this is not a problem because
            // //       we can always cast to [p-3,p-2] if we detect that the user-provided range is negative

            let range: Range<F> = (min, max);

            // If the range is already defined, skip
            let ranges = self.ranges.lock().unwrap();
            if ranges.iter().any(|r| r.range == range) {
                continue;
            }
            drop(ranges);

            // Otherwise, register the range
            let zero = F::zero();
            let byte = F::from_canonical_u8(BYTE);
            let twobytes = F::from_canonical_u16(TWOBYTES);
            // Associate to each unique range a range check type
            let r#type = if predefined && range.0 >= zero && range.1 <= twobytes {
                match range {
                    (min, max) if min == zero && max == byte => {
                        StdRangeCheckType::Valid(RangeCheckAir::U8Air)
                    }
                    (min, max) if min == zero && max == twobytes => {
                        StdRangeCheckType::Valid(RangeCheckAir::U16Air)
                    }
                    (_, max) if max <= byte => StdRangeCheckType::U8AirDouble,
                    (_, max) if max <= twobytes => StdRangeCheckType::U16AirDouble,
                    _ => panic!("Invalid predefined range"),
                }
            } else {
                StdRangeCheckType::Valid(RangeCheckAir::SpecifiedRanges)
            };

            // Update ranges
            let mut ranges = self.ranges.lock().unwrap();
            ranges.push(StdRangeItem {
                rc_type: r#type,
                range,
            });
        }
    }

    pub fn assign_values(&self, value: F, min: F, max: F) {
        // TODO: Associate this check to a type!!!
        // if value < min || value > max {
        //     log::error!(
        //         "Value {} is not in the range [min,max] = [{},{}]",
        //         value,
        //         min,
        //         max
        //     );
        //     panic!();
        // }

        // TODO: Associate this to a type!!!
        // // Cast the range to positive if it is negative
        // let (min, max) = if min < BigInt::ZERO && max < BigInt::ZERO {
        //     (
        //         min + BigInt::from(F::order()),
        //         max + BigInt::from(F::order()),
        //     )
        // } else {
        //     (min, max)
        // };

        // TODO: I think this check should only be done in debug mode!!!
        // If the range was not computed in the setup phase, error
        let ranges = self.ranges.lock().unwrap();
        let range_check = ranges.iter().find(|r| r.range == (min, max));

        if range_check.is_none() {
            log::error!("Range not found: [min,max] = [{},{}]", min, max);
            panic!();
        }

        let range_check = range_check.unwrap();
        let range = range_check.range;
        match range_check.rc_type {
            StdRangeCheckType::Valid(RangeCheckAir::U8Air) => {
                self.update_inputs(RangeCheckAir::U8Air, value);
            }
            StdRangeCheckType::Valid(RangeCheckAir::U16Air) => {
                self.update_inputs(RangeCheckAir::U16Air, value);
            }
            StdRangeCheckType::U8AirDouble => {
                self.update_inputs(RangeCheckAir::U8Air, value - range.0);
                self.update_inputs(RangeCheckAir::U8Air, range.1 - value);
            }
            StdRangeCheckType::U16AirDouble => {
                self.update_inputs(RangeCheckAir::U16Air, value - range.0);
                self.update_inputs(RangeCheckAir::U16Air, range.1 - value);
            }
            StdRangeCheckType::Valid(RangeCheckAir::SpecifiedRanges) => {
                self.update_inputs_specified(value, range.0, range.1);
            }
        }
    }

    // TODO: Update mul directly and not through the inputs!
    fn update_inputs(&self, rc_type: RangeCheckAir, value: F) {
        let mut inputs = self.inputs.lock().unwrap();
        *inputs[rc_type as usize].entry(value).or_insert(F::zero()) += F::one();

        println!("Inputs: {:?}", inputs);
    }

    fn update_inputs_specified(&self, value: F, min: F, max: F) {
        let range = (min, max);

        let mut inputs_specified = self.inputs_specified.lock().unwrap();
        let range = inputs_specified.entry(range).or_insert(HashMap::new());

        // Update the value
        *range.entry(value).or_insert(F::zero()) += F::one();
    }

    pub fn calculate_witness(
        &self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if stage == 1 {
            if let Some(air_data) = self.air_data.as_ref() {
                println!("Air data: {:?}", air_data);
                println!("Ranges: {:?}", self.ranges.lock().unwrap());
                println!("Inputs: {:?}", self.inputs.lock().unwrap());
                println!(
                    "Inputs specified: {:?}",
                    self.inputs_specified.lock().unwrap()
                );

                for air in air_data {
                    let air_name = &air.air_name;
                    let air_group_id = air.air_group_id;
                    let air_id = air.air_id;

                    log::info!(
                        "{}: Initiating witness computation for AIR '{}' at stage {}",
                        Self::MY_NAME,
                        air_name.to_string(),
                        stage
                    );

                    let air = pctx.pilout.get_air(air_group_id, air_id);
                    let num_rows = air.num_rows(); // TODO: This should be a BigUint, not a usize...

                    // Add a new air instance for the air
                    let (buffer_size, offsets) = ectx
                        .buffer_allocator
                        .as_ref()
                        .get_buffer_info(air_name.to_string(), air_id)?; // TODO: In fact, it should be the air_group_name and not the air_name
                    let mut buffer = vec![F::zero(); buffer_size as usize];
                    pctx.add_air_instance_ctx(air_group_id, air_id, Some(buffer.clone()));

                    match air_name {
                        RangeCheckAir::U8Air => {
                            // Update the multiplicity column
                            let mut inputs = self.inputs.lock().unwrap();
                            let mut trace = U8Air0Trace::map_buffer(
                                &mut buffer,
                                num_rows,
                                offsets[0] as usize,
                            )?;
                            for i in 0..num_rows {
                                trace[i].mul = *inputs[RangeCheckAir::U8Air as usize]
                                    .entry(F::from_canonical_usize(i))
                                    .or_insert(F::zero());
                            }

                            for i in 0..100 {
                                // Inputs: [{1: 64, 0: 64, 76: 64}, {1: 64, 0: 64, 128: 64}]
                                println!("U8Air0Trace[{}]: {:?}", i, trace[i].mul);
                            }
                        }
                        RangeCheckAir::U16Air => {
                            // Update the multiplicity column
                            let mut inputs = self.inputs.lock().unwrap();
                            let mut trace = U8Air0Trace::map_buffer(
                                &mut buffer,
                                num_rows,
                                offsets[0] as usize,
                            )?;
                            for i in 0..num_rows {
                                trace[i].mul = *inputs[RangeCheckAir::U16Air as usize]
                                    .entry(F::from_canonical_usize(i))
                                    .or_insert(F::zero());
                            }

                            for i in 0..100 {
                                // Inputs: [{1: 64, 0: 64, 76: 64}, {1: 64, 0: 64, 128: 64}]
                                println!("U16Air0Trace[{}]: {:?}", i, trace[i].mul);
                            }
                        }
                        RangeCheckAir::SpecifiedRanges => {
                            let inputs_specified = self.inputs_specified.lock().unwrap();
                            let mut trace = SpecifiedRanges0Trace::map_buffer(
                                &mut buffer,
                                num_rows,
                                offsets[0] as usize,
                            )?;

                            for k in 0..trace[0].mul.len() {
                                let range = inputs_specified
                                    .keys()
                                    .nth(k)
                                    .expect("Rc::calculate_trace() range not found")
                                    .clone();
                                let min = range.0;
                                let max = range.1;
                                for i in 0..num_rows {
                                    // Ranges doesn't necessarily have to be a power of two
                                    // so we must adjust the multiplicity to that case
                                    if F::from_canonical_usize(i) >= max - min + F::one() {
                                        trace[k].mul[i] = F::zero();
                                    } else {
                                        trace[k].mul[i] = *inputs_specified
                                            .get(&range)
                                            .unwrap()
                                            .clone()
                                            .entry(F::from_canonical_usize(i))
                                            .or_insert(F::zero());
                                    }
                                }
                            }
                        }
                    }

                    log::info!(
                        "{}: Completed witness computation for AIR '{}' at stage {}",
                        Self::MY_NAME,
                        air_name.to_string(),
                        stage
                    );
                }
            } else {
                log::error!("No air data provided");
                panic!();
            }
        }

        Ok(0)
    }
}
