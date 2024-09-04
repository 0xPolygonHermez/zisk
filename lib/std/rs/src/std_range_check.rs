use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    os::raw::c_void,
    sync::{Arc, Mutex},
};

use num_bigint::BigInt;
use p3_field::PrimeField;

use proofman_common::{trace, AirInstanceCtx, ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{get_hint_field, get_hint_ids_by_name, HintFieldValue};

use crate::Decider;

const BYTE: u64 = 255;
const TWOBYTES: u64 = 65535;

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

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum RangeCheckAir {
    U8Air,
    U16Air,
    SpecifiedRanges,
}

impl Display for RangeCheckAir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RangeCheckAir::U8Air => write!(f, "U8Air"),
            RangeCheckAir::U16Air => write!(f, "U16Air"),
            RangeCheckAir::SpecifiedRanges => write!(f, "SpecifiedRanges"),
        }
    }
}

pub struct RCAirData {
    air_name: RangeCheckAir,
    air_group_id: usize,
    air_id: usize,
}

#[derive(Clone)]
enum StdRangeCheckType {
    Valid(RangeCheckAir),
    U8AirDouble,
    U16AirDouble,
}

const STD_RANGE_CHECK_VARIANTS: usize = 3;
const STD_RANGE_CHECK_AIR_NAMES: [&str; STD_RANGE_CHECK_VARIANTS] =
    ["U8Air", "U16Air", "SpecifiedRanges"];

type Range = (BigInt, BigInt); // (min, max)

#[derive(Clone)]
struct StdRangeItem {
    rc_type: StdRangeCheckType,
    range: Range,
}

pub struct StdRangeCheck<F> {
    air_data: Option<Vec<RCAirData>>,
    ranges: Mutex<Vec<StdRangeItem>>,
    inputs: Mutex<[HashMap<BigInt, F>; STD_RANGE_CHECK_VARIANTS - 1]>, // value -> multiplicity
    inputs_specified: Mutex<HashMap<Range, HashMap<BigInt, F>>>, // range -> value -> multiplicity
}

impl<F: PrimeField> Decider<F> for StdRangeCheck<F> {
    fn decide(&self, pctx: &ProofCtx<F>, sctx: &SetupCtx) {
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
                    // self.register_ranges(sctx, rc_hints);
                }
            });
        });
    }
}

// impl<F: PrimeField> Decider<F> for StdRangeCheck<F> {
//     fn decide(
//         &self,
//         pctx: &mut ProofCtx<F>,
//         ectx: &ExecutionCtx,
//         sctx: &SetupCtx,
//     ) {
//         if stage == 1 && self.air_data.is_some() {
//             // Create an air instance for each range check type
//             let air_data = self.air_data.as_ref().unwrap();
//             for rc_type in air_data.iter() {
//                 let (air_name, air_group_id, air_ids) = (rc_type.air_name.clone(), rc_type.air_group_id, rc_type.air_ids);
//                 let air_id = if air_ids.len() == 1 {
//                     air_ids[0]
//                 } else {
//                     log::error!("Invalid number of air ids for range check air");
//                     panic!();
//                 };

//                 pctx.add_air_instance_ctx(air_group_id, air_id, None);

//                 if let Err(e) = self.calculate_trace(stage, air_name, air_group_id, air_id, pctx, ectx, sctx) {
//                     log::error!("Failed to calculate witness: {:?}", e);
//                     panic!();
//                 }
//             }
//         }
//     }
// }

impl<F: PrimeField> StdRangeCheck<F> {
    const MY_NAME: &'static str = "STD Range Check";

    pub fn new(air_data: Option<Vec<RCAirData>>) -> Arc<Self> {
        // Check that the provided air data is valid
        if let Some(air_data) = air_data.as_ref() {
            if air_data.len() != STD_RANGE_CHECK_VARIANTS {
                log::error!(
                    "Invalid number of range check airs: expected {}, found {}",
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
        air_instance: &mut AirInstanceCtx<F>,
        sctx: &SetupCtx,
        rc_hints: Vec<u64>,
    ) {
        for hint in rc_hints {
            let predefined =
                get_hint_field::<F>(sctx, air_instance, hint as usize, "predefined", false);
            let min = get_hint_field::<F>(sctx, air_instance, hint as usize, "min", false);
            let max = get_hint_field::<F>(sctx, air_instance, hint as usize, "max", false);

            let HintFieldValue::Field(predefined) = predefined else {
                log::error!("Predefined hint must be a field element");
                panic!();
            };
            let HintFieldValue::Field(min) = min else {
                log::error!("Min hint must be a field element");
                panic!();
            };
            let HintFieldValue::Field(max) = max else {
                log::error!("Max hint must be a field element");
                panic!();
            };

            let predefined = {
                if !predefined.is_zero() && !predefined.is_one() {
                    log::error!("Predefined hint must be either 0 or 1");
                    panic!();
                }
                predefined.is_one()
            };

            // Convert min and max to BigInt
            let mut min = BigInt::from(min.as_canonical_biguint());
            let max = BigInt::from(max.as_canonical_biguint());

            // Hint fields can only be expressed as field elements but in PIL they can be negative
            // e.g.: on input range [-3,3], we obtain the range [p-3,3] which is counterintuitive
            // we should therefore adjust the range to [-3,3]
            if min > max {
                min -= BigInt::from(F::order());
            }
            // Note: It is impossible to distinguish between [-3,-2] and [p-3,p-2] (from a bigint perspective)
            //       and, in fact, the range will be saved as [p-3,p-2]. However, this is not a problem because
            //       we can always cast to [p-3,p-2] if we detect that the user-provided range is negative

            let range: Range = (min, max);

            // If the range is already defined, skip
            let ranges = self.ranges.lock().unwrap();
            if ranges.iter().any(|r| r.range == range) {
                continue;
            }
            drop(ranges);

            // Otherwise, register the range
            let zero = BigInt::ZERO;
            let byte = BigInt::from(BYTE);
            let twobytes = BigInt::from(TWOBYTES);
            // Associate to each unique range a range check type
            let r#type = if predefined && range.0 >= zero && range.1 <= twobytes {
                match range {
                    (ref min, ref max) if *min == zero && *max == byte => {
                        StdRangeCheckType::Valid(RangeCheckAir::U8Air)
                    }
                    (ref min, ref max) if *min == zero && *max == twobytes => {
                        StdRangeCheckType::Valid(RangeCheckAir::U16Air)
                    }
                    (_, ref max) if *max <= byte => StdRangeCheckType::U8AirDouble,
                    (_, ref max) if *max <= twobytes => StdRangeCheckType::U16AirDouble,
                    _ => panic!("Invalid predefined range"),
                }
            } else {
                // Invoke "update_inputs" to enforce a specific order for the user-provided ranges
                // This is useful to avoid the need to reentry the range when computing the multiplicity column
                self.update_inputs(
                    RangeCheckAir::SpecifiedRanges,
                    None,
                    Some(range.0.clone()),
                    Some(range.1.clone()),
                );
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

    pub fn assign_values(&self, value: BigInt, min: BigInt, max: BigInt) {
        if value < min || value > max {
            log::error!(
                "Value {} is not in the range [min,max] = [{},{}]",
                value,
                min,
                max
            );
            panic!();
        }

        // Cast the range to positive if it is negative
        let (min, max) = if min < BigInt::ZERO && max < BigInt::ZERO {
            (
                min + BigInt::from(F::order()),
                max + BigInt::from(F::order()),
            )
        } else {
            (min, max)
        };

        // If the range was not part of the setup, error
        let ranges = self.ranges.lock().unwrap();
        let range_check = ranges
            .iter()
            .find(|r| r.range == (min.clone(), max.clone()))
            .cloned();

        if range_check.is_none() {
            log::error!("Range not found: [min,max] = [{},{}]", min, max);
            panic!();
        }

        let range_check = range_check.unwrap();
        let range = range_check.range;
        match range_check.rc_type {
            StdRangeCheckType::Valid(RangeCheckAir::U8Air) => {
                self.update_inputs(RangeCheckAir::U8Air, Some(value), None, None);
            }
            StdRangeCheckType::Valid(RangeCheckAir::U16Air) => {
                self.update_inputs(RangeCheckAir::U16Air, Some(value), None, None);
            }
            StdRangeCheckType::U8AirDouble => {
                self.update_inputs(
                    RangeCheckAir::U8Air,
                    Some(value.clone() - range.0.clone()),
                    None,
                    None,
                );
                self.update_inputs(
                    RangeCheckAir::U8Air,
                    Some(range.1.clone() - value),
                    None,
                    None,
                );
            }
            StdRangeCheckType::U16AirDouble => {
                self.update_inputs(
                    RangeCheckAir::U16Air,
                    Some(value.clone() - range.0.clone()),
                    None,
                    None,
                );
                self.update_inputs(
                    RangeCheckAir::U16Air,
                    Some(range.1.clone() - value),
                    None,
                    None,
                );
            }
            StdRangeCheckType::Valid(RangeCheckAir::SpecifiedRanges) => {
                self.update_inputs(
                    RangeCheckAir::SpecifiedRanges,
                    Some(value),
                    Some(range.0),
                    Some(range.1),
                );
            }
        }
    }

    fn update_inputs(
        &self,
        rc_type: RangeCheckAir,
        value: Option<BigInt>,
        min: Option<BigInt>,
        max: Option<BigInt>,
    ) {
        if rc_type != RangeCheckAir::SpecifiedRanges {
            let value = value.expect("Rc::update_inputs() value must be provided");

            let mut inputs = self.inputs.lock().unwrap();
            *inputs[rc_type as usize].entry(value).or_insert(F::zero()) += F::one();
        } else {
            let min = min.expect("Rc::update_inputs() min must be provided");
            let max = max.expect("Rc::update_inputs() max must be provided");
            let range = (min, max);

            let mut inputs_specified = self.inputs_specified.lock().unwrap();
            let range = inputs_specified.entry(range).or_insert(HashMap::new());

            if value.is_none() {
                return;
            }

            let value = value.unwrap();

            // Update the value
            *range.entry(value).or_insert(F::zero()) += F::one();
        }
    }

    pub fn calculate_witness(
        &self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if stage == 1 {
            if let Some(air_data) = self.air_data.as_ref() {
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

                    let (buffer_size, offsets) = ectx
                        .buffer_allocator
                        .as_ref()
                        .get_buffer_info(air_name.to_string(), air_id)?;
                    let mut buffer = vec![F::zero(); buffer_size as usize];
                    match air_name {
                        RangeCheckAir::U8Air => {
                            let mut inputs = self.inputs.lock().unwrap();
                            let mut trace = U8Air0Trace::map_buffer(
                                &mut buffer,
                                num_rows,
                                offsets[0] as usize,
                            )?;
                            for i in 0..num_rows {
                                trace[i].mul = *inputs[RangeCheckAir::U8Air as usize]
                                    .entry(i.into())
                                    .or_insert(F::zero());
                            }
                        }
                        RangeCheckAir::U16Air => {
                            let mut inputs = self.inputs.lock().unwrap();
                            let mut trace = U16Air0Trace::map_buffer(
                                &mut buffer,
                                num_rows,
                                offsets[0] as usize,
                            )?;
                            for i in 0..num_rows {
                                trace[i].mul = *inputs[RangeCheckAir::U16Air as usize]
                                    .entry(i.into())
                                    .or_insert(F::zero());
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
                                let min = range.0.clone();
                                let max = range.1.clone();
                                for i in 0..num_rows {
                                    // Ranges doesn't necessarily have to be a power of two
                                    // so we must adjust the multiplicity to that case
                                    if BigInt::from(i) >= &max - &min + BigInt::from(1) {
                                        trace[k].mul[i] = F::zero();
                                    } else {
                                        trace[k].mul[i] = *inputs_specified
                                            .get(&range)
                                            .unwrap()
                                            .clone()
                                            .entry(i.into())
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
            }
        }

        Ok(0)
    }
}
