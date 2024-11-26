use std::{
    hash::Hash,
    collections::HashMap,
    fmt::{Display, Debug},
    sync::{Arc, Mutex},
};

use num_traits::ToPrimitive;
use p3_field::{Field, PrimeField};
use rayon::prelude::*;

use log::debug;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{
    format_vec, get_hint_field, get_hint_field_a, get_hint_ids_by_name, mul_hint_fields, set_hint_field,
    set_hint_field_val, HintFieldOptions, HintFieldOutput,
};

use crate::{Decider, StdMode, ModeName};

type SumAirsItem = (usize, usize, Vec<u64>, Vec<u64>, Vec<u64>);

pub struct StdSum<F: Copy + Display + Hash> {
    mode: StdMode,
    sum_airs: Mutex<Vec<SumAirsItem>>, // (airgroup_id, air_id, gsum_hints, im_hints, debug_hints_data, debug_hints)
    debug_data: Option<DebugData<F>>,
}

struct BusValue<F: Copy> {
    num_proves: F,
    num_assumes: F,
    // meta data
    row_proves: usize,       // Note: For now, we assume that a value in proves is unique
    row_assumes: Vec<usize>, //       Also, multiplicity in assumes can only be one or zero
}

type DebugData<F> = Mutex<HashMap<F, HashMap<Vec<HintFieldOutput<F>>, BusValue<F>>>>; // opid -> val -> BusValue

impl<F: Field> Decider<F> for StdSum<F> {
    fn decide(&self, sctx: Arc<SetupCtx<F>>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have sum-related hints
        let air_groups = pctx.pilout.air_groups();
        let mut sum_airs_guard = self.sum_airs.lock().unwrap();
        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;

                let setup = sctx.get_setup(airgroup_id, air_id);
                let p_expressions_bin = setup.p_setup.p_expressions_bin;

                let im_hints = get_hint_ids_by_name(p_expressions_bin, "im_col");
                let gsum_hints = get_hint_ids_by_name(p_expressions_bin, "gsum_col");
                let debug_hints_data = get_hint_ids_by_name(p_expressions_bin, "gsum_member_data");
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
            debug_data: if mode.name == ModeName::Debug { Some(Mutex::new(HashMap::new())) } else { None },
        });

        wcm.register_component(std_sum.clone(), None, None);

        std_sum
    }

    fn debug(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        air_instance: &mut AirInstance<F>,
        num_rows: usize,
        debug_hints_data: Vec<u64>,
    ) {
        for hint in debug_hints_data.iter() {
            // let _name =
            //     get_hint_field::<F>(sctx, pctx, air_instance, *hint as usize, "name_piop", HintFieldOptions::default());

            let sumid =
                get_hint_field::<F>(sctx, pctx, air_instance, *hint as usize, "sumid", HintFieldOptions::default());
            if let HintFieldOutput::Field(sumid) = sumid.get(0) {
                if let Some(opids) = &self.mode.opids {
                    if !opids.contains(&sumid.as_canonical_biguint().to_u64().expect("Cannot convert to u64")) {
                        continue;
                    }
                }
            } else {
                panic!("sumid must be a field element");
            };

            let proves =
                get_hint_field::<F>(sctx, pctx, air_instance, *hint as usize, "proves", HintFieldOptions::default());

            let mul =
                get_hint_field::<F>(sctx, pctx, air_instance, *hint as usize, "selector", HintFieldOptions::default());

            let expressions = get_hint_field_a::<F>(
                sctx,
                pctx,
                air_instance,
                *hint as usize,
                "references",
                HintFieldOptions::default(),
            );

            // let _names = get_hint_field_a::<F>(
            //     sctx,
            //     &pctx.public_inputs,
            //     &pctx.challenges,
            //     air_instance,
            //     *hint as usize,
            //     "names",
            //     HintFieldOptions::default(),
            // );

            (0..num_rows).into_par_iter().for_each(|j| {
                let mut mul = match mul.get(j) {
                    HintFieldOutput::Field(mul) => mul,
                    _ => panic!("mul must be a field element"),
                };

                if !mul.is_zero() {
                    let sumid = match sumid.get(j) {
                        HintFieldOutput::Field(sumid) => sumid,
                        _ => panic!("sumid must be a field element"),
                    };

                    let is_positive = match proves.get(j) {
                        HintFieldOutput::Field(proves) => match proves {
                            p if p.is_zero() || p == -F::one() => {
                                // If it's an assume, then negate its value
                                if p == -F::one() {
                                    mul = -mul;
                                }
                                false
                            }
                            p if p.is_one() => true,
                            _ => panic!("Proves hint must be either 0, 1, or -1"),
                        },
                        _ => panic!("Proves hint must be a field element"),
                    };

                    self.update_bus_vals(num_rows, sumid, expressions.get(j), j, is_positive, mul);
                }
            });
        }
    }

    fn update_bus_vals(
        &self,
        num_rows: usize,
        opid: F,
        val: Vec<HintFieldOutput<F>>,
        row: usize,
        is_positive: bool,
        times: F,
    ) {
        let debug_data = self.debug_data.as_ref().expect("Debug data missing");
        let mut bus = debug_data.lock().expect("Bus values missing");

        let bus_opid = bus.entry(opid).or_default();

        let bus_val = bus_opid.entry(val).or_insert_with(|| BusValue {
            num_proves: F::zero(),
            num_assumes: F::zero(),
            row_proves: 0,
            row_assumes: Vec::with_capacity(num_rows),
        });

        if is_positive {
            bus_val.num_proves = times;
            bus_val.row_proves = row;
        } else {
            assert!(times.is_one());
            bus_val.num_assumes += times;
            bus_val.row_assumes.push(row);
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for StdSum<F> {
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, _ectx: Arc<ExecutionCtx<F>>, sctx: Arc<SetupCtx<F>>) {
        self.decide(sctx, pctx);
    }

    fn calculate_witness(
        &self,
        stage: u32,
        _air_instance: Option<usize>,
        pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
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

                    debug!("{}: ··· Computing witness for AIR '{}' at stage {}", Self::MY_NAME, air_name, stage);

                    let num_rows = air.num_rows();

                    if self.mode.name == ModeName::Debug {
                        self.debug(&pctx, &sctx, air_instance, num_rows, debug_hints_data.clone());
                    }

                    // Populate the im columns
                    for hint in im_hints {
                        let id = mul_hint_fields::<F>(
                            &sctx,
                            &pctx,
                            air_instance,
                            *hint as usize,
                            "reference",
                            "numerator",
                            HintFieldOptions::default(),
                            "denominator",
                            HintFieldOptions::inverse(),
                        );

                        air_instance.set_commit_calculated(id as usize);
                    }

                    // We know that at most one product hint exists
                    let gsum_hint = if gsum_hints.len() > 1 {
                        panic!("Multiple product hints found for AIR '{}'", air.name().unwrap_or("unknown"));
                    } else {
                        gsum_hints[0] as usize
                    };

                    // This call accumulates "expression" into "reference" expression and stores its last value to "result"
                    // Alternatively, this could be done using get_hint_field and set_hint_field methods and doing the accumulation in Rust,
                    // TODO: GENERALIZE CALLS
                    let direct_num = get_hint_field::<F>(
                        &sctx,
                        &pctx,
                        air_instance,
                        gsum_hint,
                        "direct_num",
                        HintFieldOptions::default(),
                    );
                    let direct_den = get_hint_field::<F>(
                        &sctx,
                        &pctx,
                        air_instance,
                        gsum_hint,
                        "direct_den",
                        HintFieldOptions::inverse(),
                    );
                    let sum_ims = get_hint_field::<F>(
                        &sctx,
                        &pctx,
                        air_instance,
                        gsum_hint,
                        "sum_ims",
                        HintFieldOptions::default(),
                    );
                    let mut gsum = get_hint_field::<F>(
                        &sctx,
                        &pctx,
                        air_instance,
                        gsum_hint,
                        "reference",
                        HintFieldOptions::default(),
                    );
                    gsum.set(0, direct_num.get(0) * direct_den.get(0) + sum_ims.get(0));
                    for i in 1..num_rows {
                        gsum.set(i, gsum.get(i - 1) + direct_num.get(i) * direct_den.get(i) + sum_ims.get(i));
                    }

                    let result = gsum.get(num_rows - 1);

                    set_hint_field::<F>(&sctx, air_instance, gsum_hint as u64, "reference", &gsum);
                    set_hint_field_val::<F>(&sctx, air_instance, gsum_hint as u64, "result", result);
                }
            }
        }
    }

    fn end_proof(&self) {
        if self.mode.name == ModeName::Debug {
            let max_values_to_print = self.mode.vals_to_print;

            let mut there_are_errors = false;
            let debug_data = self.debug_data.as_ref().expect("Debug data missing");
            let mut bus_vals = debug_data.lock().expect("Bus values missing");
            for (opid, bus) in bus_vals.iter_mut() {
                if bus.iter().any(|(_, v)| v.num_proves != v.num_assumes) {
                    if !there_are_errors {
                        there_are_errors = true;
                        log::error!("{}: Some bus values do not match.", Self::MY_NAME);
                    }
                    println!("\t► Mismatched bus values for opid {}:", opid);
                } else {
                    continue;
                }

                let mut unmatching_values2: Vec<(&Vec<HintFieldOutput<F>>, &mut BusValue<F>)> =
                    bus.iter_mut().filter(|(_, v)| v.num_proves < v.num_assumes).collect();
                let len2 = unmatching_values2.len();

                if len2 > 0 {
                    println!("\t  ⁃ There are {} unmatching values thrown as 'assume':", len2);
                }

                for (i, (val, data)) in unmatching_values2.iter_mut().enumerate() {
                    let num_proves = data.num_proves;
                    let num_assumes = data.num_assumes;
                    let diff = num_assumes - num_proves;
                    let diff = diff.as_canonical_biguint().to_usize().expect("Cannot convert to usize");
                    let row_assumes = &mut data.row_assumes;

                    row_assumes.sort();
                    let row_assumes = if max_values_to_print < diff {
                        row_assumes[..max_values_to_print].to_vec()
                    } else {
                        row_assumes[..diff].to_vec()
                    };
                    let row_assumes = row_assumes.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");

                    let diff = num_assumes - num_proves;
                    let name_str = if row_assumes.len() == 1 {
                        format!("at row {}.", row_assumes)
                    } else if max_values_to_print < row_assumes.len() {
                        format!("at rows {},...", row_assumes)
                    } else {
                        format!("at rows {}.", row_assumes)
                    };
                    let diff_str = if diff.is_one() { "time" } else { "times" };
                    println!(
                        "\t    • Value:\n\t        {}\n\t      Appears {} {} {}\n\t      Num Assumes: {}.\n\t      Num Proves: {}.",
                        format_vec(val),
                        diff,
                        diff_str,
                        name_str,
                        num_assumes,
                        num_proves
                    );

                    if i == max_values_to_print {
                        println!("\t      ...");
                        break;
                    }
                }

                if len2 > 0 {
                    println!();
                }

                let unmatching_values1: Vec<(&Vec<HintFieldOutput<F>>, &mut BusValue<F>)> =
                    bus.iter_mut().filter(|(_, v)| v.num_proves > v.num_assumes).collect();
                let len1 = unmatching_values1.len();

                if len1 > 0 {
                    println!("\t  ⁃ There are {} unmatching values thrown as 'prove':", len1);
                }

                for (i, (val, data)) in unmatching_values1.iter().enumerate() {
                    let num_proves = data.num_proves;
                    let num_assumes = data.num_assumes;
                    let row_proves = data.row_proves;

                    let diff = num_proves - num_assumes;

                    let diff_str = if diff.is_one() { "time" } else { "times" };
                    println!(
                        "\t    • Value:\n\t        {}\n\t      Appears {} {} at row {}.\n\t      Num Assumes: {}.\n\t      Num Proves: {}.",
                        format_vec(val),
                        diff,
                        diff_str,
                        row_proves,
                        num_assumes,
                        num_proves,
                    );

                    if i == max_values_to_print {
                        println!("\t      ...");
                        break;
                    }
                }

                println!();
            }
        }
    }
}
