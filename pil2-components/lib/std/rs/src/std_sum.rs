use core::panic;
use rayon::prelude::*;
use std::{
    collections::BTreeMap,
    fmt::{Display, Debug},
    sync::{Arc, Mutex},
};

use num_traits::ToPrimitive;
use p3_field::{Field, PrimeField};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{
    get_hint_field, get_hint_ids_by_name, set_hint_field, set_hint_field_val, HintFieldOptions, HintFieldOutput,
    HintFieldValue,
};

use crate::{Decider, StdMode, ModeName};

type SumAirsItem = (usize, usize, Vec<u64>, Vec<u64>, Vec<u64>, Vec<u64>);
type BusVals<F> = Vec<(usize, Vec<HintFieldOutput<F>>)>;

pub struct StdSum<F: Copy + Display> {
    mode: StdMode,
    sum_airs: Mutex<Vec<SumAirsItem>>, // (airgroup_id, air_id, gsum_hints, im_hints, debug_hints_data, debug_hints)
    debug_data: Option<DebugData<F>>,
}

struct DebugData<F: Copy> {
    bus_vals_left: Mutex<BTreeMap<F, BusVals<F>>>,  // opid -> (row, bus_val)
    bus_vals_right: Mutex<BTreeMap<F, BusVals<F>>>, // opid -> (row, bus_val)
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
                    sum_airs_guard.push((airgroup_id, air_id, im_hints, gsum_hints, debug_hints_data, debug_hints));
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
                    bus_vals_left: Mutex::new(BTreeMap::new()),
                    bus_vals_right: Mutex::new(BTreeMap::new()),
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
        debug_hints: Vec<u64>,
    ) {
        let mut past_ncols = 0;
        for hint in debug_hints_data.iter() {
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
            let proves = if let HintFieldValue::Field(proves) = proves {
                if !proves.is_zero() && !proves.is_one() {
                    log::error!("Proves hint must be either 0 or 1");
                    panic!();
                }
                proves.is_one()
            } else {
                log::error!("Proves hint must be a field element");
                panic!();
            };

            let ncols = get_hint_field::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "ncols",
                HintFieldOptions::default(),
            );
            let ncols = if let HintFieldValue::Field(ncols) = ncols {
                ncols.as_canonical_biguint().to_usize().expect("Cannot convert to usize")
            } else {
                log::error!("Proves hint must be a field element");
                panic!();
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

            let mut bus_vals = Vec::new();
            for hint in debug_hints[past_ncols..(past_ncols + ncols)].iter() {
                let col = get_hint_field::<F>(
                    sctx,
                    &pctx.public_inputs,
                    &pctx.challenges,
                    air_instance,
                    *hint as usize,
                    "reference",
                    HintFieldOptions::default(),
                );

                bus_vals.push(col);
            }
            past_ncols += ncols;

            for j in 0..num_rows {
                let mul = if let HintFieldOutput::Field(mul) = mul.get(j) {
                    mul
                } else {
                    panic!("mul must be a field element");
                };

                // TODO: bus_throws should be used as a counter of the value, not repeating the calls multiple times...
                if !mul.is_zero() {
                    let bus_throws = mul.as_canonical_biguint().to_usize().expect("Cannot convert to usize");

                    let sumid = if let HintFieldOutput::Field(sumid) = sumid.get(j) {
                        sumid
                    } else {
                        panic!("sumid must be a field element");
                    };

                    let bus_value = bus_vals.iter().map(|col| col.get(j)).collect();
                    self.update_bus_vals(sumid, bus_value, j, !proves, bus_throws);
                }
            }
        }
    }

    fn update_bus_vals(&self, opid: F, val: Vec<HintFieldOutput<F>>, row: usize, is_num: bool, times: usize) {
        let mut bus_vals;
        let mut other_bus_vals;
        if is_num {
            bus_vals = self.debug_data.as_ref().unwrap().bus_vals_left.lock().unwrap();
            other_bus_vals = self.debug_data.as_ref().unwrap().bus_vals_right.lock().unwrap();
        } else {
            bus_vals = self.debug_data.as_ref().unwrap().bus_vals_right.lock().unwrap();
            other_bus_vals = self.debug_data.as_ref().unwrap().bus_vals_left.lock().unwrap();
        }

        let bus_vals_map = bus_vals.entry(opid).or_insert(Vec::new());

        for _ in 0..times {
            if let Some(idx) = bus_vals_map.iter().position(|(_, v)| *v == val) {
                bus_vals_map.remove(idx);
            } else {
                other_bus_vals.entry(opid).or_insert(Vec::new()).push((row, val.clone()));
            }
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

            for (airgroup_id, air_id, im_hints, gsum_hints, debug_hints_data, debug_hints) in sum_airs.iter() {
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
                        self.debug(&pctx, &sctx, air_instance, num_rows, debug_hints_data.clone(), debug_hints.clone());
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
            let max_values_to_print = 5;

            let bus_vals_left = self.debug_data.as_ref().unwrap().bus_vals_left.lock().unwrap();
            let bus_vals_right = self.debug_data.as_ref().unwrap().bus_vals_right.lock().unwrap();
            if !bus_vals_left.is_empty() || !bus_vals_right.is_empty() {
                log::error!("{}: Some bus values do not match.", Self::MY_NAME);

                println!("\t ► Unmatching bus values thrown as 'assume':");
                for (opid, vals) in bus_vals_right.iter() {
                    println!("\t  ⁃ Opid {}: {} values", opid, vals.len());
                    print_rows(vals, max_values_to_print);
                }

                println!("\t ► Unmatching bus values thrown as 'prove':");
                for (opid, vals) in bus_vals_left.iter() {
                    println!("\t  ⁃ Opid {}: {} values", opid, vals.len());
                    print_rows(vals, max_values_to_print);
                }
            }
        }

        fn print_rows<F: Field>(vals: &Vec<(usize, Vec<HintFieldOutput<F>>)>, max_values_to_print: usize) {
            let num_values = vals.len();

            if max_values_to_print >= num_values {
                for (row, val) in vals {
                    println!("\t    • Row {}: {:?}", row, val);
                }
                return;
            }

            // Print the first max_values_to_print
            for (row, val) in vals[..max_values_to_print].into_iter() {
                println!("\t    • Row {}: {:?}", row, val);
            }

            println!("\t      ...");

            // Print the last max_values_to_print
            for (row, val) in vals[num_values - max_values_to_print..].into_iter() {
                println!("\t    • Row {}: {:?}", row, val);
            }

            println!();
        }
    }
}
