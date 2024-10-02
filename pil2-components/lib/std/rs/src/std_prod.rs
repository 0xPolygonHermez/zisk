use core::panic;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    fmt::Display,
};

use num_traits::ToPrimitive;
use p3_field::{Field, PrimeField};
use rayon::prelude::*;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{
    format_vec, get_hint_field, get_hint_field_a, get_hint_ids_by_name, set_hint_field, set_hint_field_val,
    HintFieldOptions, HintFieldOutput, HintFieldValue,
};

use crate::{Decider, StdMode, ModeName};

type ProdAirsItem = (usize, usize, Vec<u64>, Vec<u64>);

pub struct StdProd<F: Copy + Display> {
    mode: StdMode,
    prod_airs: Mutex<Vec<ProdAirsItem>>, // (airgroup_id, air_id, gprod_hints, debug_hints_data, debug_hints)
    debug_data: Option<DebugData<F>>,
}

struct BusValue<F: Copy> {
    num_proves: F,
    num_assumes: F,
    // meta data
    row_proves: Vec<usize>,
    row_assumes: Vec<usize>,
}

type DebugData<F> = Mutex<HashMap<F, HashMap<Vec<HintFieldOutput<F>>, BusValue<F>>>>; // opid -> val -> BusValue

impl<F: Field> Decider<F> for StdProd<F> {
    fn decide(&self, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have prod-related hints
        for airgroup in pctx.pilout.air_groups() {
            for air in airgroup.airs() {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;

                let setup = sctx.get_partial_setup(airgroup_id, air_id).expect("REASON");
                let p_setup = (&setup.p_setup).into();

                let gprod_hints = get_hint_ids_by_name(p_setup, "gprod_col");
                let debug_hints_data = get_hint_ids_by_name(p_setup, "gprod_member_data");
                if !gprod_hints.is_empty() {
                    // Save the air for latter witness computation
                    self.prod_airs.lock().unwrap().push((airgroup_id, air_id, gprod_hints, debug_hints_data));
                }
            }
        }
    }
}

impl<F: PrimeField> StdProd<F> {
    const MY_NAME: &'static str = "STD Prod";

    pub fn new(mode: StdMode, wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let std_prod = Arc::new(Self {
            mode: mode.clone(),
            prod_airs: Mutex::new(Vec::new()),
            debug_data: if mode.name == ModeName::Debug { Some(Mutex::new(HashMap::new())) } else { None },
        });

        wcm.register_component(std_prod.clone(), None, None);

        std_prod
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

            let opid = get_hint_field::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "opid",
                HintFieldOptions::default(),
            );
            let opid = if let HintFieldValue::Field(opid) = opid {
                if let Some(opids) = &self.mode.opids {
                    if !opids.contains(&opid.as_canonical_biguint().to_u64().expect("Cannot convert to u64")) {
                        continue;
                    }
                }

                opid
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

            let selector = get_hint_field::<F>(
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

            // let _names = get_hint_field::<F>(
            //     sctx,
            //     &pctx.public_inputs,
            //     &pctx.challenges,
            //     air_instance,
            //     *hint as usize,
            //     "names",
            //     HintFieldOptions::default(),
            // );

            (0..num_rows).into_par_iter().for_each(|j| {
                let sel = if let HintFieldOutput::Field(selector) = selector.get(j) {
                    if !selector.is_zero() && !selector.is_one() {
                        log::error!("Selector must be either 0 or 1");
                        panic!();
                    }
                    selector.is_one()
                } else {
                    log::error!("Selector must be a field element");
                    panic!();
                };

                if sel {
                    self.update_bus_vals(num_rows, opid, expressions.get(j), j, proves);
                }
            });
        }
    }

    fn update_bus_vals(&self, num_rows: usize, opid: F, val: Vec<HintFieldOutput<F>>, row: usize, is_num: bool) {
        let debug_data = self.debug_data.as_ref().expect("Debug data missing");
        let mut bus = debug_data.lock().expect("Bus values missing");

        let bus_opid = bus.entry(opid).or_default();

        let bus_val = bus_opid.entry(val).or_insert_with(|| BusValue {
            num_proves: F::zero(),
            num_assumes: F::zero(),
            row_proves: Vec::with_capacity(num_rows),
            row_assumes: Vec::with_capacity(num_rows),
        });

        if is_num {
            bus_val.num_proves += F::one();
            bus_val.row_proves.push(row);
        } else {
            bus_val.num_assumes += F::one();
            bus_val.row_assumes.push(row);
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for StdProd<F> {
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
            let prod_airs = self.prod_airs.lock().unwrap();

            for (airgroup_id, air_id, gprod_hints, debug_hints_data) in prod_airs.iter() {
                let air_instance_ids = pctx.air_instance_repo.find_air_instances(*airgroup_id, *air_id);

                for air_instance_id in air_instance_ids {
                    let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
                    let air_instance = &mut air_instances_vec[air_instance_id];

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

                    // We know that at most one product hint exists
                    let gprod_hint = if gprod_hints.len() > 1 {
                        panic!("Multiple product hints found for AIR '{}'", air.name().unwrap_or("unknown"));
                    } else {
                        gprod_hints[0] as usize
                    };

                    // Use the hint to populate the gprod column
                    let mut gprod = get_hint_field::<F>(
                        &sctx,
                        &pctx.public_inputs,
                        &pctx.challenges,
                        air_instance,
                        gprod_hint,
                        "reference",
                        HintFieldOptions::dest(),
                    );
                    let num = get_hint_field::<F>(
                        &sctx,
                        &pctx.public_inputs,
                        &pctx.challenges,
                        air_instance,
                        gprod_hint,
                        "numerator",
                        HintFieldOptions::default(),
                    );
                    let den = get_hint_field::<F>(
                        &sctx,
                        &pctx.public_inputs,
                        &pctx.challenges,
                        air_instance,
                        gprod_hint,
                        "denominator",
                        HintFieldOptions::default(),
                    );

                    gprod.set(0, num.get(0) / den.get(0));
                    for i in 1..num_rows {
                        gprod.set(i, gprod.get(i - 1) * (num.get(i) / den.get(i)));
                    }

                    // set the computed gprod column and its associated airgroup_val
                    set_hint_field(&sctx, air_instance, gprod_hint as u64, "reference", &gprod);
                    set_hint_field_val(&sctx, air_instance, gprod_hint as u64, "result", gprod.get(num_rows - 1));
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

                // TODO: Sort unmatching values by the row
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

                    let name_str = if row_assumes.len() == 1 {
                        format!("at row {}.", row_assumes)
                    } else {
                        if max_values_to_print < row_assumes.len() {
                            format!("at rows {},...", row_assumes)
                        } else {
                            format!("at rows {}.", row_assumes)
                        }
                    };
                    let diff_str = if diff == 1 { "time" } else { "times" };
                    println!(
                        "\t    • Value:\n\t        {}\n\t      Appears {} {} {}",
                        format_vec(val),
                        diff,
                        diff_str,
                        name_str
                    );

                    if i == max_values_to_print {
                        println!("\t      ...");
                        break;
                    }
                }

                println!();

                // TODO: Sort unmatching values by the row
                let mut unmatching_values1: Vec<(&Vec<HintFieldOutput<F>>, &mut BusValue<F>)> =
                    bus.iter_mut().filter(|(_, v)| v.num_proves > v.num_assumes).collect();
                let len1 = unmatching_values1.len();

                if len1 > 0 {
                    println!("\t  ⁃ There are {} unmatching values thrown as 'prove':", len1);
                }

                for (i, (val, data)) in unmatching_values1.iter_mut().enumerate() {
                    let num_proves = data.num_proves;
                    let num_assumes = data.num_assumes;
                    let diff = num_proves - num_assumes;
                    let diff = diff.as_canonical_biguint().to_usize().expect("Cannot convert to usize");
                    let row_proves = &mut data.row_proves;

                    row_proves.sort();
                    let row_proves = if max_values_to_print < diff {
                        row_proves[..max_values_to_print].to_vec()
                    } else {
                        row_proves[..diff].to_vec()
                    };
                    let row_proves = row_proves.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");

                    let name_str = if row_proves.len() == 1 {
                        format!("at row {}.", row_proves)
                    } else {
                        if max_values_to_print < row_proves.len() {
                            format!("at rows {},...", row_proves)
                        } else {
                            format!("at rows {}.", row_proves)
                        }
                    };
                    println!(
                        "\t    • Value\n\t        {}\n\t      Appears {} times {}",
                        format_vec(val),
                        diff,
                        name_str
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
