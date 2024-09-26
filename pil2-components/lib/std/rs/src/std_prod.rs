use core::panic;
use std::{
    collections::BTreeMap,
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

use crate::{Decider, StdMode};

type ProdAirsItem = (usize, usize, Vec<u64>, Vec<u64>, Vec<u64>);
pub struct StdProd<F: Copy> {
    mode: StdMode,
    prod_airs: Mutex<Vec<ProdAirsItem>>, // (airgroup_id, air_id, gprod_hints, debug_hints_data, debug_hints)
    bus_vals_num: Option<Mutex<BTreeMap<F, Vec<(usize, Vec<HintFieldOutput<F>>)>>>>, // opid -> (row, bus_val)
    bus_vals_den: Option<Mutex<BTreeMap<F, Vec<(usize, Vec<HintFieldOutput<F>>)>>>>, // opid -> (row, bus_val)
}

impl<F: PrimeField> Decider<F> for StdProd<F> {
    fn decide(&self, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have prod-related hints
        for airgroup in pctx.pilout.air_groups() {
            for air in airgroup.airs() {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.get_setup(airgroup_id, air_id).expect("REASON");
                let gprod_hints = get_hint_ids_by_name(setup.p_setup, "gprod_col");
                let debug_hints_data = get_hint_ids_by_name(setup.p_setup, "gprod_member_data");
                let debug_hints = get_hint_ids_by_name(setup.p_setup, "gprod_member");
                if !gprod_hints.is_empty() {
                    // Save the air for latter witness computation
                    self.prod_airs.lock().unwrap().push((
                        airgroup_id,
                        air_id,
                        gprod_hints,
                        debug_hints_data,
                        debug_hints,
                    ));
                }
            }
        }
    }
}

impl<F: PrimeField> StdProd<F> {
    const MY_NAME: &'static str = "STD Prod";

    pub fn new(mode: StdMode, wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let std_prod = Arc::new(Self {
            mode,
            prod_airs: Mutex::new(Vec::new()),
            bus_vals_num: if mode == StdMode::Debug { Some(Mutex::new(BTreeMap::new())) } else { None },
            bus_vals_den: if mode == StdMode::Debug { Some(Mutex::new(BTreeMap::new())) } else { None },
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
        debug_hints: Vec<u64>,
    ) {
        for (i, hint) in debug_hints_data.iter().enumerate() {
            let opid = get_hint_field::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "opid",
                HintFieldOptions::default(),
            );
            let HintFieldValue::Field(opid) = opid else {
                log::error!("Opid hint must be a field element");
                panic!();
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

            let selector = get_hint_field::<F>(
                sctx,
                &pctx.public_inputs,
                &pctx.challenges,
                air_instance,
                *hint as usize,
                "selector",
                HintFieldOptions::default(),
            );

            let mut bus_vals = BTreeMap::new();
            for (j, hint) in debug_hints[i * ncols..(i + 1) * ncols].iter().enumerate() {
                let col = get_hint_field::<F>(
                    sctx,
                    &pctx.public_inputs,
                    &pctx.challenges,
                    air_instance,
                    *hint as usize,
                    "reference",
                    HintFieldOptions::default(),
                );

                bus_vals.insert(j, col);
            }

            for j in 0..num_rows {
                let sel = selector.get(j);
                if let HintFieldOutput::Field(sel) = sel {
                    if sel.is_zero() {
                        continue;
                    }
                    let bus_value = bus_vals.values().filter_map(|v| Some(v.get(j))).collect();
                    self.update_bus_vals(opid, bus_value, j, proves);
                }
            }
        }
    }

    fn update_bus_vals(&self, opid: F, val: Vec<HintFieldOutput<F>>, row: usize, is_num: bool) {
        let mut bus_vals;
        let mut other_bus_vals;
        if is_num {
            bus_vals = self.bus_vals_den.as_ref().unwrap().lock().unwrap();
            other_bus_vals = self.bus_vals_num.as_ref().unwrap().lock().unwrap();
        } else {
            bus_vals = self.bus_vals_num.as_ref().unwrap().lock().unwrap();
            other_bus_vals = self.bus_vals_den.as_ref().unwrap().lock().unwrap();
        }

        let bus_vals_map = bus_vals.entry(opid).or_insert(Vec::new());

        if let Some(idx) = bus_vals_map.iter().position(|(_, v)| *v == val) {
            bus_vals_map.remove(idx);
        } else {
            other_bus_vals.entry(opid).or_insert(Vec::new()).push((row, val));
        }

        if bus_vals_map.is_empty() {
            bus_vals.remove(&opid);
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

            for (airgroup_id, air_id, gprod_hints, debug_hints_data, debug_hints) in prod_airs.iter() {
                let air_instance_ids = pctx.air_instance_repo.find_air_instances(*airgroup_id, *air_id);

                for air_instance_id in air_instance_ids {
                    let air_instances_vec = &mut pctx.air_instance_repo.air_instances.write().unwrap();
                    let air_instance = &mut air_instances_vec[air_instance_id];

                    // Get the air associated with the air_instance
                    let airgroup_id = air_instance.airgroup_id;
                    let air_id = air_instance.air_id;
                    let air = pctx.pilout.get_air(airgroup_id, air_id);
                    let air_name = air.name().unwrap_or("unknown");

                    log::info!(
                        "{}: Initiating witness computation for AIR '{}' at stage {}",
                        Self::MY_NAME,
                        air_name,
                        stage
                    );

                    let num_rows = air.num_rows();

                    if self.mode == StdMode::Debug {
                        self.debug(&pctx, &sctx, air_instance, num_rows, debug_hints_data.clone(), debug_hints.clone());
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

                    log::info!(
                        "{}: Completed witness computation for AIR '{}' at stage {}",
                        Self::MY_NAME,
                        air.name().unwrap_or("unknown"),
                        stage
                    );
                }
            }
        }
    }

    fn end_proof(&self) {
        if self.mode == StdMode::Debug {
            let max_values_to_print = 5;

            let bus_vals_num = self.bus_vals_num.as_ref().unwrap().lock().unwrap();
            let bus_vals_den = self.bus_vals_den.as_ref().unwrap().lock().unwrap();
            if !bus_vals_num.is_empty() || !bus_vals_den.is_empty() {
                log::error!("{}: Some bus values do not match.", Self::MY_NAME);

                println!("\t ► Unmatching bus values thrown as 'prove':");
                for (opid, vals) in bus_vals_num.iter() {
                    println!("\t  ⁃ Opid {}: {} values", opid, vals.len());
                    let left_to_print = print_rows(vals.iter().map(|(row, val)| (*row, val)), max_values_to_print);
                    if left_to_print == 0 {
                        println!("\t      ...");
                    } else {
                        break;
                    }
                    print_rows(vals.iter().rev().map(|(row, val)| (*row, val)), max_values_to_print);
                    println!();
                }

                println!("\t ► Unmatching bus values thrown as 'assume':");
                for (opid, vals) in bus_vals_den.iter() {
                    println!("\t  ⁃ Opid {}: {} values", opid, vals.len());
                    let left_to_print = print_rows(vals.iter().map(|(row, val)| (*row, val)), max_values_to_print);
                    if left_to_print == 0 {
                        println!("\t      ...");
                    } else {
                        break;
                    }
                    print_rows(vals.iter().rev().map(|(row, val)| (*row, val)), max_values_to_print);
                    println!();
                }
            }
        }

        fn print_rows<'a, I, F>(vals: I, max_values_to_print: usize) -> usize
        where
            I: Iterator<Item = (usize, &'a Vec<HintFieldOutput<F>>)>,
            F: Field,
        {
            let mut n = max_values_to_print;
            for (row, val) in vals {
                println!("\t    • Row {}: {:?}", row, val);

                n -= 1;
                if n == 0 {
                    break;
                }
            }

            n
        }
    }
}
