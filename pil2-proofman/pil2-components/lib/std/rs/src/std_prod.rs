use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use witness::WitnessComponent;
use proofman_common::{AirInstance, ModeName, ProofCtx, SetupCtx};
use proofman_hints::{
    get_hint_field_gc_constant_a, get_hint_field, get_hint_field_a, acc_mul_hint_fields, update_airgroupvalue,
    get_hint_ids_by_name, HintFieldOptions, HintFieldValue, HintFieldValuesVec,
};

use crate::{
    extract_field_element_as_usize, get_global_hint_field_constant_as, get_hint_field_constant_a_as_string,
    get_hint_field_constant_as_field, get_hint_field_constant_as_string, get_row_field_value, print_debug_info,
    update_debug_data, AirComponent, DebugData,
};

pub struct StdProd<F: PrimeField> {
    pctx: Arc<ProofCtx<F>>,
    stage_wc: Option<Mutex<u32>>,
    debug_data: Option<DebugData<F>>,
}

impl<F: PrimeField> AirComponent<F> for StdProd<F> {
    const MY_NAME: &'static str = "STD Prod";

    fn new(
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx>,
        _airgroup_id: Option<usize>,
        _air_id: Option<usize>,
    ) -> Arc<Self> {
        // Retrieve the std_prod_users hint ID
        let std_prod_users_id = get_hint_ids_by_name(sctx.get_global_bin(), "std_prod_users");

        // Initialize std_prod with the extracted data
        Arc::new(Self {
            pctx: pctx.clone(),
            stage_wc: match std_prod_users_id.is_empty() {
                true => None,
                false => {
                    // Get the "stage_wc" hint
                    let stage_wc =
                        get_global_hint_field_constant_as::<u32, F>(sctx.clone(), std_prod_users_id[0], "stage_wc");
                    Some(Mutex::new(stage_wc))
                }
            },
            debug_data: if pctx.options.debug_info.std_mode.name == ModeName::Debug {
                Some(Mutex::new(HashMap::new()))
            } else {
                None
            },
        })
    }

    fn debug_mode(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
        air_instance: &mut AirInstance<F>,
        air_instance_id: usize,
        num_rows: usize,
        debug_data_hints: Vec<u64>,
    ) {
        let debug_data = self.debug_data.as_ref().expect("Debug data missing");
        let airgroup_id = air_instance.airgroup_id;
        let air_id = air_instance.air_id;

        // Process each debug hint
        for &hint in debug_data_hints.iter() {
            // Extract hint fields
            let name_piop = get_hint_field_constant_as_string::<F>(
                sctx,
                airgroup_id,
                air_id,
                hint as usize,
                "name_piop",
                HintFieldOptions::default(),
            );

            let name_expr = get_hint_field_constant_a_as_string::<F>(
                sctx,
                airgroup_id,
                air_id,
                hint as usize,
                "name_expr",
                HintFieldOptions::default(),
            );

            let opid = get_hint_field_constant_as_field::<F>(
                sctx,
                airgroup_id,
                air_id,
                hint as usize,
                "busid",
                HintFieldOptions::default(),
            );

            // If opids are specified, then only update the bus if the opid is in the list
            if !pctx.options.debug_info.std_mode.opids.is_empty()
                && !pctx
                    .options
                    .debug_info
                    .std_mode
                    .opids
                    .contains(&opid.as_canonical_biguint().to_u64().expect("Cannot convert to u64"))
            {
                continue;
            }

            let is_global = get_hint_field_constant_as_field::<F>(
                sctx,
                airgroup_id,
                air_id,
                hint as usize,
                "is_global",
                HintFieldOptions::default(),
            );

            let proves =
                get_hint_field::<F>(sctx, pctx, air_instance, hint as usize, "proves", HintFieldOptions::default());

            let sel: HintFieldValue<F> =
                get_hint_field::<F>(sctx, pctx, air_instance, hint as usize, "selector", HintFieldOptions::default());

            let expressions = get_hint_field_a::<F>(
                sctx,
                pctx,
                air_instance,
                hint as usize,
                "expressions",
                HintFieldOptions::default(),
            );

            let deg_expr = get_hint_field_constant_as_field::<F>(
                sctx,
                airgroup_id,
                air_id,
                hint as usize,
                "deg_expr",
                HintFieldOptions::default(),
            );

            let deg_sel = get_hint_field_constant_as_field::<F>(
                sctx,
                airgroup_id,
                air_id,
                hint as usize,
                "deg_sel",
                HintFieldOptions::default(),
            );

            // If both the expresion and the mul are of degree zero, then simply update the bus once
            if deg_expr.is_zero() && deg_sel.is_zero() {
                update_bus(
                    &name_piop,
                    &name_expr,
                    airgroup_id,
                    air_id,
                    air_instance_id,
                    opid,
                    &proves,
                    &sel,
                    &expressions,
                    0,
                    debug_data,
                    is_global.is_one(),
                );
            }
            // Otherwise, update the bus for each row
            else {
                for j in 0..num_rows {
                    update_bus(
                        &name_piop,
                        &name_expr,
                        airgroup_id,
                        air_id,
                        air_instance_id,
                        opid,
                        &proves,
                        &sel,
                        &expressions,
                        j,
                        debug_data,
                        false,
                    );
                }
            }

            #[allow(clippy::too_many_arguments)]
            fn update_bus<F: PrimeField>(
                name_piop: &str,
                name_expr: &[String],
                airgroup_id: usize,
                air_id: usize,
                air_instance_id: usize,
                opid: F,
                proves: &HintFieldValue<F>,
                sel: &HintFieldValue<F>,
                expressions: &HintFieldValuesVec<F>,
                row: usize,
                debug_data: &DebugData<F>,
                is_global: bool,
            ) {
                let mut sel = get_row_field_value(sel, row, "sel");
                if sel.is_zero() {
                    return;
                }

                let proves = match get_row_field_value(proves, row, "proves") {
                    p if p.is_zero() || p == F::neg_one() => {
                        // If it's an "assume", negate its value
                        if p == F::neg_one() {
                            sel = -sel;
                        }
                        false
                    }
                    p if p.is_one() => true,
                    _ => panic!("Proves hint must be either 0, 1, or -1"),
                };

                update_debug_data(
                    debug_data,
                    name_piop,
                    name_expr,
                    opid,
                    expressions.get(row),
                    airgroup_id,
                    air_id,
                    air_instance_id,
                    row,
                    proves,
                    sel,
                    is_global,
                );
            }
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for StdProd<F> {
    fn calculate_witness(&self, stage: u32, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        let stage_wc = self.stage_wc.as_ref();
        if stage_wc.is_none() {
            return;
        }

        if stage == *stage_wc.unwrap().lock().unwrap() {
            // Get the number of product check users and their airgroup and air IDs
            let std_prod_users = get_hint_ids_by_name(sctx.get_global_bin(), "std_prod_users")[0];

            let num_users = get_global_hint_field_constant_as::<usize, F>(sctx.clone(), std_prod_users, "num_users");
            let airgroup_ids = get_hint_field_gc_constant_a::<F>(sctx.clone(), std_prod_users, "airgroup_ids", false);
            let air_ids = get_hint_field_gc_constant_a::<F>(sctx.clone(), std_prod_users, "air_ids", false);

            // Process each product check user
            for i in 0..num_users {
                let airgroup_id = extract_field_element_as_usize(&airgroup_ids.values[i], "airgroup_id");
                let air_id = extract_field_element_as_usize(&air_ids.values[i], "air_id");

                // Get all air instances ids for this airgroup and air_id
                let global_instance_ids = pctx.air_instance_repo.find_air_instances(airgroup_id, air_id);
                for global_instance_id in global_instance_ids {
                    // Retrieve all air instances
                    let air_instances = &mut pctx.air_instance_repo.air_instances.write().unwrap();
                    let air_instance = air_instances.get_mut(&global_instance_id).unwrap();

                    if !air_instance.prover_initialized {
                        continue;
                    }

                    // Get the air associated with the air_instance
                    let airgroup_id = air_instance.airgroup_id;
                    let air_id = air_instance.air_id;
                    let air_name = &pctx.global_info.airs[airgroup_id][air_id].name;

                    let setup = sctx.get_setup(airgroup_id, air_id);
                    let p_expressions_bin = setup.p_setup.p_expressions_bin;

                    log::debug!("{}: ··· Computing witness for AIR '{}' at stage {}", Self::MY_NAME, air_name, stage);

                    let num_rows = pctx.global_info.airs[airgroup_id][air_id].num_rows;

                    let gprod_hints = get_hint_ids_by_name(p_expressions_bin, "gprod_col");
                    let debug_data_hints = get_hint_ids_by_name(p_expressions_bin, "gprod_debug_data");

                    // Debugging, if enabled
                    if pctx.options.debug_info.std_mode.name == ModeName::Debug {
                        let air_instance_id = pctx.dctx_find_air_instance_id(global_instance_id);
                        self.debug_mode(
                            &pctx,
                            &sctx,
                            air_instance,
                            air_instance_id,
                            num_rows,
                            debug_data_hints.clone(),
                        );
                    }

                    // We know that at most one product hint exists
                    let gprod_hint = if gprod_hints.len() > 1 {
                        panic!("Multiple product hints found for AIR '{}'", air_name);
                    } else {
                        gprod_hints[0] as usize
                    };

                    // This call calculates "numerator" / "denominator" and accumulates it into "reference". Its last value is stored into "result"
                    // Alternatively, this could be done using get_hint_field and set_hint_field methods and calculating the operations in Rust,
                    acc_mul_hint_fields::<F>(
                        &sctx,
                        &pctx,
                        air_instance,
                        gprod_hint,
                        "reference",
                        "result",
                        "numerator_air",
                        "denominator_air",
                        HintFieldOptions::default(),
                        HintFieldOptions::inverse(),
                        false,
                    );

                    update_airgroupvalue::<F>(
                        &sctx,
                        &pctx,
                        air_instance,
                        gprod_hint,
                        "result",
                        "numerator_direct",
                        "denominator_direct",
                        HintFieldOptions::default(),
                        HintFieldOptions::inverse(),
                        false,
                    );
                }
            }

            // TODO: Process each direct update to the bus
            // when airgroup hints are available
        }
    }

    fn end_proof(&self) {
        // Print debug info if in debug mode
        if self.pctx.options.debug_info.std_mode.name == ModeName::Debug {
            let pctx = &self.pctx;
            let name = Self::MY_NAME;
            let max_values_to_print = pctx.options.debug_info.std_mode.n_vals;
            let print_to_file = pctx.options.debug_info.std_mode.print_to_file;
            let debug_data = self.debug_data.as_ref().expect("Debug data missing");
            print_debug_info(pctx, name, max_values_to_print, print_to_file, debug_data);
        }
    }
}
