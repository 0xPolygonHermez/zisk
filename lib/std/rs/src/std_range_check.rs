use std::collections::HashMap;

use num_bigint::BigInt;
use p3_field::Field;
use pilout::{pilout::{BasicAir, Hint}, pilout_proxy::PilOutProxy};

use proofman_common::{trace,AirInstanceCtx, ExecutionCtx, ProofCtx};

const BYTE: u64 = 255;
const TWOBYTES: u64 = 65535;

#[derive(Debug, PartialEq)]
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

type Range = (BigInt, BigInt);

pub struct StdRangeItem {
    rc_type: StdRangeCheckType,
    range: Range, // (min, max)
}

pub struct StdRangeCheck<F> {
    ranges: Vec<StdRangeItem>,
    inputs_specified: HashMap<Range, HashMap<F, F>>,     // (min, max) -> value -> multiplicity
    inputs: [HashMap<F, F>; STD_RANGE_CHECK_VARIANTS-1], // value -> multiplicity
}

impl<F: Field + Copy + Clone + PartialOrd + PartialEq>
    StdRangeCheck<F>
{
    const MY_NAME: &'static str = "STD Range Check";

    pub fn new() -> Self {
        Self {
            ranges: Vec::new(),
            inputs_specified: HashMap::new(),
            inputs: core::array::from_fn(|_| HashMap::new()),
        }
    }

    pub fn register_ranges(&self, air_group_id: u32, air_id: u32, pilout: &PilOutProxy) {
        let rc_hints: Vec<&Hint> = pilout
            .pilout
            .hints
            .iter()
            .filter(|hint| hint.subproof_id == Some(air_group_id) && hint.air_id == Some(air_id))
            .collect();

        for hint in rc_hints {
            let predefined: F = get_hint_field_c(hint, "predefined");
            let mut min: F = get_hint_field_c(hint, "min");
            let max: F = get_hint_field_c(hint, "max");

            // Hint fields can only be expressed as field elements but in PIL they can be negative
            // e.g.: on input range [-3,3], we obtain min = F::order() - 3 and max = 3
            // we should therefore adjust the range to [min,max] = [-3,3]

            if min > max {
                min = min as BigInt - F::order();
            }

            let range: Range = (min, max);

            // If the range is already defined, skip
            if self.ranges.iter().any(|r| r.range == range) {
                continue;
            }

            // Associate to each unique range a range check type
            let r#type = if predefined && min >= BigInt::ZERO && max <= BigInt::from(TWOBYTES) {
                match (min, max) {
                    (0, BYTE) => StdRangeCheckType::U8Air,
                    (0, TWOBYTES) => StdRangeCheckType::U16Air,
                    (_, max) if max <= BYTE => StdRangeCheckType::U8AirDouble,
                    (_, max) if max <= TWOBYTES => StdRangeCheckType::U16AirDouble,
                    _ => panic!("Invalid predefined range"),
                }
            } else {
                self.proves(
                    StdRangeCheckType::SpecifiedRanges,
                    None,
                    Some(min),
                    Some(max),
                ); // To create the range in a specific order
                StdRangeCheckType::SpecifiedRanges
            };

            self.ranges.push(StdRangeItem {
                rc_type: r#type,
                range,
            });
        }
    }

    pub fn assign_values(
        &mut self,
        value: F,
        min: F,
        max: F,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if value < min || value > max {
            return Err("Value is out of range".into());
        }

        let range_check = self.ranges.iter().find(|r| r.range == (min, max));

        if range_check.is_none() {
            // format!("Range not found: [min,max] = [{},{}]", min, max)
            return Err("Range check values not found".into());
        }

        let range_check = range_check.unwrap();
        let range = range_check.range;
        match range_check.rc_type {
            StdRangeCheckType::U8Air => {
                self.proves(StdRangeCheckType::U8Air, Some(value), None, None);
            }
            StdRangeCheckType::U16Air => {
                self.proves(StdRangeCheckType::U16Air, Some(value), None, None);
            }
            StdRangeCheckType::U8AirDouble => {
                self.proves(StdRangeCheckType::U8Air, Some(value - range.0), None, None);
                self.proves(StdRangeCheckType::U8Air, Some(range.1 - value), None, None);
            }
            StdRangeCheckType::U16AirDouble => {
                self.proves(StdRangeCheckType::U16Air, Some(value - range.0), None, None);
                self.proves(StdRangeCheckType::U16Air, Some(range.1 - value), None, None);
            }
            StdRangeCheckType::SpecifiedRanges => {
                self.proves(
                    StdRangeCheckType::SpecifiedRanges,
                    Some(value),
                    Some(range.0),
                    Some(range.1),
                );
            }
        }

        Ok(())
    }

    fn proves(
        &mut self,
        rc_type: StdRangeCheckType,
        value: Option<F>,
        min: Option<F>,
        max: Option<F>,
    ) {
        if rc_type != StdRangeCheckType::SpecifiedRanges {
            let value = value.expect("Rc::proves() value must be provided");

            let inputs = self.inputs[rc_type as usize].entry(value).or_insert(F::zero());
            *inputs += F::one();
        } else {
            let range = (
                min.expect("Rc::proves() min must be provided"),
                max.expect("Rc::proves() max must be provided"),
            );
            let inputs_specified = self.inputs_specified.entry(range).or_insert(HashMap::new());
            if value.is_none() {
                return;
            }
            let value = value.unwrap();
            if value > range.1 {
                // This only happens when min is negative and max is positive
                value = value - F::order();
            }

            let inputs_specified = inputs_specified.entry(value).or_insert(F::zero());
            *inputs_specified += F::one();
        }
    }

    fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx,) {
        log::info!("{} ··· Starting witness computation stage {}", Self::MY_NAME, stage);

        if stage == 1 {
            // Find airs that are range checks, which are those in STD_RANGE_CHECK_AIR_NAMES
            let mut rc_airs: Vec<&BasicAir> = Vec::new();
            for air_group in pctx.pilout.air_groups() {
                let airs = air_group.airs().iter().filter(|air| {
                    if let Some(name) = &air.name {
                        STD_RANGE_CHECK_AIR_NAMES.contains(&name.as_str())
                    } else {
                        false
                    }
                }).collect();
                rc_airs.extend(airs);
            }

            // If there is some range check air, we feed its multiplicity column for their SINGLE instance
            for air in rc_airs {
                let num_rows = air.num_rows();

                let mut buffer = Vec::with_capacity(N);

                let offset = get_offset_c();

                let trace = U8Air0Trace::<F>::map_buffer(&mut buffer, N, offset).unwrap();

                let mut air_instances = pctx.air_instances.write().unwrap();

                air_instances.push(AirInstanceCtx {
                    air_group_id: air.air_group_id(),
                    air_id: air.air_id(),
                    buffer: Some(trace.buffer.unwrap()),
                });

                // TODO: Now, we should wait until ALL components using the range check have been executed

                // self.inputs

                //     const mul = airInstance.wtnsPols[subproof].mul;
                //     if (Array.isArray(mul[0])) {
                //         const keys = Object.keys(this.inputs[subproof]);
                //         if (keys.length === mul[0].length) throw new Error(`[${this.name}]`, `Too many ranges.`);
                //         for (let k = 0; k < mul.length; k++) {
                //             const key = keys[k].split(':');
                //             const min = BigInt(key[0]);
                //             const max = BigInt(key[1]);
                //             // Ranges doesn't necessarily have to be a power of two
                //             // so we must adjust the multiplicity to that case
                //             for (let i = 0; i < N; i++) {
                //                 if (BigInt(i) >= max - min + 1n) {
                //                     mul[k][i] = 0n;
                //                 } else {
                //                     mul[k][i] = this.inputs[subproof][keys[k]][BigInt(i)+min] ?? 0n;
                //                 }
                //             }
                //         }
                //     } else {
                //         for (let i = 0; i < N; i++) {
                //             mul[i] = this.inputs[subproof][i] ?? 0n;
                //         }
                //     }
            }
        } else if stage == 2 {
            //     const instanceToProcess = this.proofCtx.getAirInstancesBySubproofIdAirId(subproofId,airInstance.airId)[airInstance.instanceId];

            //     const hints = this.proofCtx.setup.setup[subproofId][airInstance.airId].expressionsInfo.hintsInfo;

            //     const hint_gsum = hints.find(h => h.name === 'gsum_col');

            //     if (hint_gsum) {
            //         await this.components['Sum']._witnessComputation(stageId, subproofId, instanceToProcess, publics, hint_gsum);
            //     } else {
            //         throw new Error(`[${this.name}]`, `Hint not found.`);
            //     }
        }

        log::info!("StdRngCk ··· Finishing witness computation stage {}", stage);
    }
}
