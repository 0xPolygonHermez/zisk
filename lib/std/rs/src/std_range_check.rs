use std::{collections::HashMap, fmt::Debug};

use num_bigint::BigInt;
use p3_field::PrimeField;

use proofman_common::{trace, AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_hints::{get_hint_field, get_hint_ids_by_name, HintFieldValue};
use proofman_setup::SetupCtx;

use crate::Decider;

const BYTE: u64 = 255;
const TWOBYTES: u64 = 65535;

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum StdRangeCheckType {
    U8Air,
    U16Air,
    U8AirDouble,
    U16AirDouble,
    SpecifiedRanges,
}

const STD_RANGE_CHECK_VARIANTS: usize = 3;
const STD_RANGE_CHECK_AIR_NAMES: [&str; STD_RANGE_CHECK_VARIANTS] =
    ["U8Air", "U16Air", "SpecifiedRanges"];

trace!(U8Air0Row, U8Air0Trace<F> {
    mul: F,
});

trace!(U16Air0Row, U16Air0Trace<F> {
    mul: F,
});

trace!(SpecifiedRanges0Row, SpecifiedRanges0Trace<F> {
    mul: [F; 32], // TODO: This number cannot be hardcorded, it depens on the air that instantiates the range check
});

#[derive(Debug, PartialEq)]
struct Range {
    min: BigInt,
    max: BigInt,
}

struct StdRangeItem {
    rc_type: StdRangeCheckType,
    range: Range, // (min, max)
}

enum InputType<F> {
    Range(HashMap<Range, HashMap<BigInt, F>>), // (min, max) -> value -> multiplicity
    NoRange(HashMap<BigInt, F>),               // value -> multiplicity
}

pub struct StdRangeCheck<F> {
    ranges: Vec<StdRangeItem>,
    inputs: HashMap<StdRangeCheckType, InputType<F>>, // name -> InputType
    setup_done: bool,
}

impl<F: PrimeField> Decider<F> for StdRangeCheck<F> {
    fn decide(
        &self,
        stage: u32,
        air_instance_idx: usize,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        if stage == 1 && self.setup_done {
            if let Err(e) = self.calculate_trace(stage, air_instance_idx, pctx, ectx, sctx) {
                log::error!("Failed to calculate witness: {:?}", e);
                panic!();
            }
        }
    }
}

impl<F: PrimeField> StdRangeCheck<F> {
    const MY_NAME: &'static str = "STD Range Check";

    pub fn new() -> Self {
        Self {
            ranges: Vec::new(),
            inputs: HashMap::with_capacity(STD_RANGE_CHECK_VARIANTS),
            setup_done: false,
        }
    }

    pub fn register_ranges(&self, air_group_id: usize, air_id: usize, sctx: &SetupCtx) {
        // Get the range check hints of the air
        let setup = sctx.get_setup(air_group_id, air_id).expect("REASON");
        let rc_hints = get_hint_ids_by_name(setup, "range_check");

        if rc_hints.is_empty() {
            log::error!("No range check hints found, but they are required");
            panic!();
        }

        for hint in rc_hints {
            let predefined = get_hint_field::<F>(setup, hint as usize, "predefined", false);
            let min = get_hint_field::<F>(setup, hint as usize, "min", false);
            let max = get_hint_field::<F>(setup, hint as usize, "max", false);

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

            let range = Range { min, max };

            // If the range is already defined, skip
            if self.ranges.iter().any(|r| r.range == range) {
                continue;
            }

            let zero = BigInt::ZERO;
            let byte = BigInt::from(BYTE);
            let twobytes = BigInt::from(TWOBYTES);
            // Associate to each unique range a range check type
            let r#type = if predefined && min >= zero && max <= twobytes {
                match range {
                    Range { min: zero, max: byte } => StdRangeCheckType::U8Air,
                    Range { min: zero, max: twobytes } if max <= twobytes => StdRangeCheckType::U16Air,
                    Range { min: _, max } if max <= byte => StdRangeCheckType::U8AirDouble,
                    Range { min: _, max } if max <= twobytes => StdRangeCheckType::U16AirDouble,
                    _ => panic!("Invalid predefined range"),
                }
            } else {
                // Invoke "update_inputs" to enforce a specific order for the user-provided ranges
                // This is useful to avoid the need to reentry the range
                self.update_inputs(
                    StdRangeCheckType::SpecifiedRanges,
                    None,
                    Some(min),
                    Some(max),
                );
                StdRangeCheckType::SpecifiedRanges
            };

            self.ranges.push(StdRangeItem {
                rc_type: r#type,
                range,
            });
        }

        self.setup_done = true;
    }

    pub fn assign_values(
        &mut self,
        value: BigInt,
        min: BigInt,
        max: BigInt,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if value < min || value > max {
            return Err(format!(
                "Value {} is not in the range [min,max] = [{},{}]",
                value, min, max
            )
            .into());
        }

        // Cast the range to positive if it is negative
        let (min, max) = if min < BigInt::ZERO && max < BigInt::ZERO {
            (min + BigInt::from(F::order()), max + BigInt::from(F::order()))
        } else {
            (min, max)
        };

        let range_check = self.ranges.iter().find(|r| r.range == Range { min, max });

        if range_check.is_none() {
            return Err(format!("Range not found: [min,max] = [{},{}]", min, max).into());
        }

        let range_check = range_check.unwrap();
        let range: Range = range_check.range;
        match range_check.rc_type {
            StdRangeCheckType::U8Air => {
                self.update_inputs(StdRangeCheckType::U8Air, Some(value), None, None);
            }
            StdRangeCheckType::U16Air => {
                self.update_inputs(StdRangeCheckType::U16Air, Some(value), None, None);
            }
            StdRangeCheckType::U8AirDouble => {
                self.update_inputs(StdRangeCheckType::U8Air, Some(value - range.min), None, None);
                self.update_inputs(StdRangeCheckType::U8Air, Some(range.max - value), None, None);
            }
            StdRangeCheckType::U16AirDouble => {
                self.update_inputs(StdRangeCheckType::U16Air, Some(value - range.min), None, None);
                self.update_inputs(StdRangeCheckType::U16Air, Some(range.max - value), None, None);
            }
            StdRangeCheckType::SpecifiedRanges => {
                self.update_inputs(
                    StdRangeCheckType::SpecifiedRanges,
                    Some(value),
                    Some(range.min),
                    Some(range.max),
                );
            }
        }

        Ok(())
    }

    fn update_inputs(
        &mut self,
        rc_type: StdRangeCheckType,
        value: Option<BigInt>,
        min: Option<BigInt>,
        max: Option<BigInt>,
    ) {
        self.inputs.entry(rc_type).or_insert_with(|| match rc_type {
            StdRangeCheckType::U8Air | StdRangeCheckType::U16Air => {
                InputType::NoRange(HashMap::new())
            }
            StdRangeCheckType::SpecifiedRanges => InputType::Range(HashMap::new()),
            _ => {
                panic!("Unexpected StdRangeCheckType variant");
            }
        });
        if rc_type != StdRangeCheckType::SpecifiedRanges {
            let value = value.expect("Rc::update_inputs() value must be provided");

            let inputs = self.inputs

            let inputs = self.inputs[rc_type as usize]
                .entry(value)
                .or_insert(F::zero());
            *inputs += F::one();
        } else {
            let range = (
                min.expect("Rc::update_inputs() min must be provided"),
                max.expect("Rc::update_inputs() max must be provided"),
            );

            let inputs_specified = self.inputs_specified.entry(range).or_insert(HashMap::new());
            if value.is_none() {
                return;
            }
            let value = value.unwrap();
            // TODO: Not necessary!
            // if value > range.1 {
            //     // This only happens when min is negative and max is positive
            //     value = value - F::order();
            // }

            let inputs_specified = inputs_specified.entry(value).or_insert(F::zero());
            *inputs_specified += F::one();
        }
    }

    fn calculate_trace(
        &self,
        stage: u32,
        air_instance: usize, // Can I assume that I have an air_instance?
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if stage != 1 {
            panic!("STD Range Check must be executed on stage 1");
        }

        log::info!(
            "{} ··· Starting calculating trace stage {}",
            Self::MY_NAME,
            stage
        );

        let air_instances = pctx.air_instances.read().unwrap();
        let air_instance: &AirInstanceCtx<F> = &air_instances[air_instance];

        // Get the air associated with the air instance
        let air = pctx.pilout.get_air(air_instance.air_group_id, air_instance.air_id);
        let air_name = air.name.unwrap();

        let rc_air = STD_RANGE_CHECK_AIR_NAMES.iter().find(|&&name| name == air_name);

        // If it is not a range check air, we return
        if rc_air.is_none() {
            return Ok(());
        }

        // Otherwise, we feed its multiplicity column for their SINGLE instance
        let (buffer_size, offsets) =
            ectx.buffer_allocator.as_ref().get_buffer_info(air_name, air.air_id)?;

        let mut buffer = vec![F::zero(); buffer_size as usize];

        let num_rows = air.num_rows(); // TODO: This should be a BigUint, not a usize...

        // TODO: Do it generic!
        // U8Air
        let mut trace = U8Air0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize)?;

        for i in 0..num_rows {
            trace[i].mul = *self.inputs[StdRangeCheckType::U8Air as usize].entry(i.into()).or_insert(F::zero());
        }

        // U16Air
        let mut trace = U16Air0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize)?;

        for i in 0..num_rows {
            trace[i].mul = *self.inputs[StdRangeCheckType::U16Air as usize].entry(i.into()).or_insert(F::zero());
        }

        // SpecifiedRanges
        let mut trace = SpecifiedRanges0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize)?;

        for k in 0..trace[0].mul.len() {
            let range = self.inputs_specified.keys().nth(k).unwrap();
            let min = range.0;
            let max = range.1;
            for i in 0..num_rows {
                // Ranges doesn't necessarily have to be a power of two
                // so we must adjust the multiplicity to that case
                if BigInt::from(i) >= max - min + BigInt::from(1) {
                    trace[k].mul[i] = F::zero();
                } else {
                    trace[k].mul[i] = *self.inputs_specified.entry(range.clone()).or_insert(HashMap::new()).entry(i.into()).or_insert(F::zero());
                }
            }
        }

        log::info!(
            "{} ··· Finishing calculating trace stage {}",
            Self::MY_NAME,
            stage
        );

        Ok(())
    }
}
