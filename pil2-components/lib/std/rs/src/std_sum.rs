use core::panic;
use rayon::prelude::*;
use std::{
    collections::BTreeMap,
    fmt::{Display, Debug},
    sync::{Arc, Mutex},
};

use num_traits::ToPrimitive;
use p3_field::{Field, PrimeField};
use rayon::prelude::*;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{
    get_hint_field, get_hint_field_a, get_hint_ids_by_name, set_hint_field, set_hint_field_val, HintFieldOptions,
    HintFieldOutput, HintFieldValue,
};

use crate::{Decider, StdMode, ModeName};

type SumAirsItem = (usize, usize, Vec<u64>, Vec<u64>, Vec<u64>);
type BusVals<F> = Vec<(usize, F, Vec<HintFieldOutput<F>>)>;

pub struct StdSum<F: Copy + Display> {
    mode: StdMode,
    sum_airs: Mutex<Vec<SumAirsItem>>, // (airgroup_id, air_id, gsum_hints, im_hints, debug_hints_data, debug_hints)
    debug_data: Option<DebugData<F>>,
}

struct DebugData<F: Copy> {
    bus_vals_positive: Mutex<BTreeMap<F, BusVals<F>>>, // opid -> (row, multiplicity, bus_val)
    bus_vals_negative: Mutex<BTreeMap<F, BusVals<F>>>, // opid -> (row, multiplicity, bus_val)
}

impl<F: Field> Decider<F> for StdSum<F> {
    fn decide(&self, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have sum-related hints
        let air_groups = pctx.pilout.air_groups();
        let mut sum_airs_guard = self.sum_airs.lock().unwrap();
        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;

                let setup = sctx.get_partial_setup(airgroup_id, air_id).expect("REASON");
                let p_setup = (&setup.p_setup).into();

                let im_hints = get_hint_ids_by_name(p_setup, "im_col");
                let gsum_hints = get_hint_ids_by_name(p_setup, "gsum_col");
                let debug_hints_data = get_hint_ids_by_name(p_setup, "gsum_member_data");
                let debug_hints = get_hint_ids_by_name(p_setup, "gsum_member");
                if !gsum_hints.is_empty() {
                    // Save the air for latter witness computation
                    sum_airs_guard.push((airgroup_id, air_id, im_hints, gsum_hints, debug_hints_data));
                }
            });
        });
    }
}

impl<F: Copy + Debug + PrimeField> StdSum<F> {
    const MY_NAME: &'static str = "STD Sum ";

    pub fn new(mode: StdMode, wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let std_sum = Arc::new(Self {
            mode: mode.clone(),
            sum_airs: Mutex::new(Vec::new()),
            debug_data: if mode.name == ModeName::Debug {
                Some(DebugData {
                    bus_vals_positive: Mutex::new(BTreeMap::new()),
                    bus_vals_negative: Mutex::new(BTreeMap::new()),
                })
            } else {
                None
            },
        });

        wcm.register_component(std_sum.clone(), None, None);

        std_sum
    }

    fn debug(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
        air_instance: &mut AirInstance<F>,
        num_rows: usize,
        debug_hints_data: Vec<u64>,
    ) {
        for hint in debug_hints_data.iter() {
            let _name = get_hint_field::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "name_piop",
                HintFieldOptions::default(),
            );

            let sumid = get_hint_field::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "sumid",
                HintFieldOptions::default(),
            );
            if let HintFieldOutput::Field(sumid) = sumid.get(0) {
                if let Some(opids) = &self.mode.opids {
                    if !opids.contains(&sumid.as_canonical_biguint().to_u64().expect("Cannot convert to u64")) {
                        continue;
                    }
                }
            } else {
                panic!("sumid must be a field element");
            };

            let proves = get_hint_field::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "proves",
                HintFieldOptions::default(),
            );
            let is_positive = match proves {
                HintFieldValue::Field(proves) => {
                    assert!(proves.is_zero() || proves.is_one(), "Proves hint must be either 0 or 1");
                    proves.is_one()
                }
                _ => {
                    log::error!("Proves hint must be a field element");
                    panic!("Proves hint must be a field element");
                }
            };

            let mul = get_hint_field::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "selector",
                HintFieldOptions::default(),
            );

            let expressions = get_hint_field_a::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "references",
                HintFieldOptions::default(),
            );

            let _names = get_hint_field_a::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "names",
                HintFieldOptions::default(),
            );

            for j in 0..num_rows {
                if (j % 10000) == 0 {
                    println!("Row {} / {}", j, num_rows);
                }
                let mul = match mul.get(j) {
                    HintFieldOutput::Field(mul) => mul,
                    _ => panic!("mul must be a field element"),
                };

                if !mul.is_zero() {
                    let sumid = match sumid.get(j) {
                        HintFieldOutput::Field(sumid) => sumid,
                        _ => panic!("sumid must be a field element"),
                    };

                    self.update_bus_vals(sumid, expressions.get(j), j, is_positive, mul);
                }
            }

            // // TODO: Do it in parallel!
            // (0..num_rows).into_par_iter().chunks(10000).for_each(|chunk| {
            //     for j in chunk {
            //         let mul = match mul.get(j) {
            //             HintFieldOutput::Field(mul) => mul,
            //             _ => panic!("mul must be a field element"),
            //         };
            
            //         if !mul.is_zero() {
            //             let sumid = match sumid.get(j) {
            //                 HintFieldOutput::Field(sumid) => sumid,
            //                 _ => panic!("sumid must be a field element"),
            //             };

            //             self.update_bus_vals(sumid, expressions.get(j), j, is_positive, mul);
            //         }
            //     }
            // });
        }
    }

    fn update_bus_vals(&self, opid: F, val: Vec<HintFieldOutput<F>>, row: usize, is_positive: bool, times: F) {
        let debug_data = self.debug_data.as_ref().expect("Debug data should exist");

        let (mut bus_vals, mut other_bus_vals) = if is_positive {
            let bus_vals = debug_data.bus_vals_negative.lock().expect("Negative bus values should exist");
            let other_bus_vals = debug_data.bus_vals_positive.lock().expect("Positive bus values should exist");
            (bus_vals, other_bus_vals)
        } else {
            let bus_vals = debug_data.bus_vals_positive.lock().expect("Positive bus values should exist");
            let other_bus_vals = debug_data.bus_vals_negative.lock().expect("Negative bus values should exist");
            (bus_vals, other_bus_vals)
        };

        let bus_vals_map = bus_vals.entry(opid).or_insert(Vec::new());

        if let Some((idx, (_, t, _))) = bus_vals_map.iter().enumerate().find(|(_, (_, _, v))| *v == val) {
            let diff = times - *t;
            if times > *t {
                bus_vals_map[idx].1 = F::zero();
                other_bus_vals.entry(opid).or_insert(Vec::new()).push((row, diff, val));
            } else {
                bus_vals_map[idx].1 -= times;
            }

            if bus_vals_map[idx].1.is_zero() {
                bus_vals_map.remove(idx);
            }
        } else {
            other_bus_vals.entry(opid).or_insert(Vec::new()).push((row, times, val));
        }

        if bus_vals_map.is_empty() {
            bus_vals.remove(&opid);
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for StdSum<F> {
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, _ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.decide(sctx, pctx);
    }

    fn calculate_witness(
        &self,
        stage: u32,
        _air_instance: Option<usize>,
        pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        if stage == 2 {
            let sum_airs = self.sum_airs.lock().unwrap();

            for (airgroup_id, air_id, im_hints, gsum_hints, debug_hints_data) in sum_airs.iter() {
                let air_instance_ids = pctx.air_instance_repo.find_air_instances(*airgroup_id, *air_id);

                for air_instance_id in air_instance_ids {
                    let air_instaces_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();

                    let air_instance = &mut air_instaces_vec[air_instance_id];

                    // Get the air associated with the air_instance
                    let airgroup_id = air_instance.airgroup_id;
                    let air_id = air_instance.air_id;
                    let air = pctx.pilout.get_air(airgroup_id, air_id);
                    let air_name = air.name().unwrap_or("unknown");

                    log::info!("{}: ··· Computing witness for AIR '{}' at stage {}", Self::MY_NAME, air_name, stage);

                    let num_rows = air.num_rows();

                    if self.mode.name == ModeName::Debug {
                        self.debug(&pctx, &sctx, air_instance, num_rows, debug_hints_data.clone());
                    }

                    // Populate the im columns
                    for hint in im_hints {
                        let mut im = get_hint_field::<F>(
                            &sctx,
                            &pctx.public_inputs,
                            &pctx.challenges,
                            air_instance,
                            *hint as usize,
                            "reference",
                            HintFieldOptions::dest(),
                        );
                        let num = get_hint_field::<F>(
                            &sctx,
                            &pctx.public_inputs,
                            &pctx.challenges,
                            air_instance,
                            *hint as usize,
                            "numerator",
                            HintFieldOptions::default(),
                        );
                        let den = get_hint_field::<F>(
                            &sctx,
                            &pctx.public_inputs,
                            &pctx.challenges,
                            air_instance,
                            *hint as usize,
                            "denominator",
                            HintFieldOptions::inverse(),
                        );

                        // Apply a map&reduce strategy to compute the division
                        // TODO! Explore how to do it in only one step
                        // Step 1: Compute the division in parallel
                        let results: Vec<HintFieldOutput<F>> =
                            (0..num_rows).into_par_iter().map(|i| num.get(i) * den.get(i)).collect(); // Collect results into a vector
                                                                                                      // Step 2: Store the results in 'im'
                        for (i, &value) in results.iter().enumerate() {
                            im.set(i, value);
                        }
                        set_hint_field(&sctx, air_instance, *hint, "reference", &im);
                    }

                    // We know that at most one product hint exists
                    let gsum_hint = if gsum_hints.len() > 1 {
                        panic!("Multiple product hints found for AIR '{}'", air.name().unwrap_or("unknown"));
                    } else {
                        gsum_hints[0] as usize
                    };

                    // Use the hint to populate the gsum column
                    let mut gsum = get_hint_field::<F>(
                        &sctx,
                        &pctx.public_inputs,
                        &pctx.challenges,
                        air_instance,
                        gsum_hint,
                        "reference",
                        HintFieldOptions::dest(),
                    );
                    let expr = get_hint_field::<F>(
                        &sctx,
                        &pctx.public_inputs,
                        &pctx.challenges,
                        air_instance,
                        gsum_hint,
                        "expression",
                        HintFieldOptions::default(),
                    );

                    gsum.set(0, expr.get(0));
                    for i in 1..num_rows {
                        // TODO: We should perform the following division in batch using div_lib
                        gsum.set(i, gsum.get(i - 1) + expr.get(i));
                    }

                    // set the computed gsum column and its associated airgroup_val
                    set_hint_field(&sctx, air_instance, gsum_hint as u64, "reference", &gsum);
                    set_hint_field_val(&sctx, air_instance, gsum_hint as u64, "result", gsum.get(num_rows - 1));
                }
            }
        }
    }

    fn end_proof(&self) {
        if self.mode.name == ModeName::Debug {
            let max_values_to_print = self.mode.vals_to_print;

            let bus_vals_positive = self.debug_data.as_ref().unwrap().bus_vals_positive.lock().unwrap();
            let bus_vals_negative = self.debug_data.as_ref().unwrap().bus_vals_negative.lock().unwrap();
            if !bus_vals_positive.is_empty() || !bus_vals_negative.is_empty() {
                log::error!("{}: Some bus values do not match.", Self::MY_NAME);

                println!("\t ► Unmatching bus values thrown as 'assume':");
                for (opid, vals) in bus_vals_negative.iter() {
                    let name_vals = if vals.len() == 1 { "value" } else { "values" };
                    println!("\t  ⁃ Opid {}: {} {name_vals}", opid, vals.len());
                    print_rows(vals, max_values_to_print);
                }

                println!("\t ► Unmatching bus values thrown as 'prove':");
                for (opid, vals) in bus_vals_positive.iter() {
                    let name_vals = if vals.len() == 1 { "value" } else { "values" };
                    println!("\t  ⁃ Opid {}: {} {name_vals}", opid, vals.len());
                    print_rows(vals, max_values_to_print);
                }
            }
        }

        fn print_rows<F: Field>(vals: &Vec<(usize, F, Vec<HintFieldOutput<F>>)>, max_values_to_print: usize) {
            let num_values = vals.len();

            if max_values_to_print >= num_values {
                for (row, mul, val) in vals {
                    let name_reps = if mul.is_one() { "repetition" } else { "repetitions" };
                    println!("\t    • Row {}, with {} {name_reps}: {:?}", row, mul, val);
                }
                println!();
                return;
            }

            // Print the first max_values_to_print
            for (row, mul, val) in vals[..max_values_to_print].into_iter() {
                let name_reps = if mul.is_one() { "repetition" } else { "repetitions" };
                println!("\t    • Row {}, with {} {name_reps}: {:?}", row, mul, val);
            }

            println!("\t      ...");

            // Print the last max_values_to_print
            let diff = num_values - max_values_to_print;
            let rem_len = if diff < max_values_to_print { max_values_to_print } else { diff };
            for (row, mul, val) in vals[rem_len..].into_iter() {
                let name_reps = if mul.is_one() { "repetition" } else { "repetitions" };
                println!("\t    • Row {}, with {} {name_reps}: {:?}", row, mul, val);
            }

            println!();
        }
    }
}
