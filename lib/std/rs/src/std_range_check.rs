use std::{collections::HashMap, hash::Hash};

use p3_field::AbstractField;
use pilout::{pilout::Hint, pilout_proxy::PilOutProxy};

use proofman_common::{ExecutionCtx, ProofCtx};

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

pub struct StdRangeItem<F> {
    rc_type: StdRangeCheckType,
    range: (F, F), // (min, max)
}

pub struct StdRangeCheck<F> {
    ranges: Vec<StdRangeItem<F>>,
    inputs: [HashMap<(F, F, F), u64>; STD_RANGE_CHECK_VARIANTS], // (min, max) -> multiplicity
    inputs_specified: HashMap<F, u64>,                           // value -> multiplicity
}

impl<F: AbstractField + Copy + Clone + PartialEq + Eq + Hash + core::ops::Sub<Output = F>>
    StdRangeCheck<F>
{
    pub fn new() -> Self {
        Self {
            ranges: Vec::new(),
            inputs: core::array::from_fn(|_| HashMap::new()),
            inputs_specified: HashMap::new(),
        }
    }

    pub fn setup(&self, air_group_id: u32, air_id: u32, pilout: &PilOutProxy) {
        let rc_hints: Vec<&Hint> = pilout
            .pilout
            .hints
            .iter()
            .filter(|hint| hint.subproof_id == Some(air_group_id) && hint.air_id == Some(air_id))
            .collect();

        for hint in rc_hints {
            let predefined = get_hint_field_c(hint, "predefined");
            let min = get_hint_field_c(hint, "min");
            let max = get_hint_field_c(hint, "max");

            let range = (min, max);

            let r#type = if predefined && min >= 0 && max <= TWOBYTES {
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
        let range_check = self.ranges.iter().find(|range| range.range == (min, max));

        if range_check.is_none() {
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
        match rc_type {
            StdRangeCheckType::SpecifiedRanges => {
                let value = value.expect("Rc::proves() value must be provided");

                let inputs_specified = self.inputs_specified.entry(value).or_insert(0);
                *inputs_specified += 1;
            }
            _ => {
                let range = (
                    min.expect("Rc::proves() min must be provided"),
                    max.expect("Rc::proves() max must be provided"),
                    value.expect("Rc::proves() value must be provided"),
                );
                let inputs = self.inputs[rc_type as usize].entry(range).or_insert(0);
                *inputs += 1;
            }
        };
    }

    fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        log::info!("StdRngCk ··· Starting witness computation stage {}", stage);

        if stage == 1 {
            let mut available_airs = Vec::new();
            for air_group in pctx.pilout.air_groups() {
                let airs = air_group.airs().iter().filter(|air| {
                    if let Some(name) = &air.name {
                        STD_RANGE_CHECK_AIR_NAMES.contains(&name.as_str())
                    } else {
                        false
                    }
                });
                available_airs.extend(airs);
            }

            for air in available_airs {
                // Create a AirInstanceCtx for each available air
                let num_rows = air.num_rows();

                // NOT necessary in this code
                //     const subproof = this.subproofs.find(subproof => airInstance.wtnsPols[subproof]);
                //     if (!subproof) {
                //         throw new Error(`[${this.name}] Subproof not found.`);
                //     }

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
