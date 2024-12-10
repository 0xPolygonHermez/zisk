use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ModeName, ProofCtx, SetupCtx, StdMode};
use proofman_hints::{
    get_hint_field, get_hint_field_a, get_hint_field_constant, get_hint_field_constant_a,
    get_hint_ids_by_name, HintFieldOptions, HintFieldOutput, HintFieldValue, HintFieldValuesVec,
};

use crate::{print_debug_info, update_debug_data, DebugData, Decider};

type ProdAirsItem = (usize, usize, Vec<u64>, Vec<u64>); // (airgroup_id, air_id, gprod_hints, debug_hints_data, debug_hints)

pub struct StdProd<F: PrimeField> {
    mode: StdMode,
    prod_airs: Mutex<Vec<ProdAirsItem>>,
    debug_data: Option<DebugData<F>>,
}

impl<F: PrimeField> Decider<F> for StdProd<F> {
    fn decide(&self, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have prod-related hints
        for airgroup in pctx.pilout.air_groups() {
            for air in airgroup.airs() {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;

                let setup = sctx.get_setup(airgroup_id, air_id);
                let p_expressions_bin = setup.p_setup.p_expressions_bin;

                let gprod_hints = get_hint_ids_by_name(p_expressions_bin, "gprod_col");
                let debug_hints_data = get_hint_ids_by_name(p_expressions_bin, "gprod_member_data");
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
        let debug_data = self.debug_data.as_ref().expect("Debug data missing");
        let airgroup_id = air_instance.airgroup_id;
        let air_id = air_instance.air_id;
        let instance_id = air_instance.air_instance_id.unwrap_or_default();

        for hint in debug_hints_data.iter() {
            let _name_piop = get_hint_field_constant::<F>(
                sctx,
                airgroup_id,
                air_id,
                *hint as usize,
                "name_piop",
                HintFieldOptions::default(),
            );

            let _name_expr = get_hint_field_constant_a::<F>(
                sctx,
                airgroup_id,
                air_id,
                *hint as usize,
                "name_expr",
                HintFieldOptions::default(),
            );

            let opid =
                get_hint_field::<F>(sctx, pctx, air_instance, *hint as usize, "opid", HintFieldOptions::default());
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

            let HintFieldValue::Field(is_global) = get_hint_field_constant::<F>(
                sctx,
                airgroup_id,
                air_id,
                *hint as usize,
                "is_global",
                HintFieldOptions::default(),
            ) else {
                log::error!("is_global hint must be a field element");
                panic!();
            };

            let HintFieldValue::Field(proves) = get_hint_field_constant::<F>(
                sctx,
                airgroup_id,
                air_id,
                *hint as usize,
                "proves",
                HintFieldOptions::default(),
            ) else {
                log::error!("proves hint must be a field element");
                panic!();
            };
            let proves = if proves.is_zero() {
                false
            } else if proves.is_one() {
                true
            } else {
                log::error!("Proves hint must be either 0 or 1");
                panic!();
            };

            let selector =
                get_hint_field::<F>(sctx, pctx, air_instance, *hint as usize, "selector", HintFieldOptions::default());

            let expressions = get_hint_field_a::<F>(
                sctx,
                pctx,
                air_instance,
                *hint as usize,
                "expressions",
                HintFieldOptions::default(),
            );

            let HintFieldValue::Field(deg_expr) = get_hint_field_constant::<F>(
                sctx,
                airgroup_id,
                air_id,
                *hint as usize,
                "deg_expr",
                HintFieldOptions::default(),
            ) else {
                log::error!("deg_expr hint must be a field element");
                panic!();
            };

            let HintFieldValue::Field(deg_sel) = get_hint_field_constant::<F>(
                sctx,
                airgroup_id,
                air_id,
                *hint as usize,
                "deg_sel",
                HintFieldOptions::default(),
            ) else {
                log::error!("deg_sel hint must be a field element");
                panic!();
            };

            if deg_expr.is_zero() && deg_sel.is_zero() {
                update_bus(
                    airgroup_id,
                    air_id,
                    instance_id,
                    opid,
                    proves,
                    &selector,
                    &expressions,
                    0,
                    debug_data,
                    is_global.is_one(),
                );
            } else {
                // Otherwise, update the bus for each row
                for j in 0..num_rows {
                    update_bus(
                        airgroup_id,
                        air_id,
                        instance_id,
                        opid,
                        proves,
                        &selector,
                        &expressions,
                        j,
                        debug_data,
                        false,
                    );
                }
            }

            #[allow(clippy::too_many_arguments)]
            fn update_bus<F: PrimeField>(
                airgroup_id: usize,
                air_id: usize,
                instance_id: usize,
                opid: F,
                proves: bool,
                selector: &HintFieldValue<F>,
                expressions: &HintFieldValuesVec<F>,
                row: usize,
                debug_data: &DebugData<F>,
                is_global: bool,
            ) {
                let sel = if let HintFieldOutput::Field(selector) = selector.get(row) {
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
                    update_debug_data(
                        debug_data,
                        opid,
                        expressions.get(row),
                        airgroup_id,
                        air_id,
                        instance_id,
                        row,
                        proves,
                        F::one(),
                        is_global,
                    );
                }
            }
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

                    log::debug!("{}: ··· Computing witness for AIR '{}' at stage {}", Self::MY_NAME, air_name, stage);

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

                    // This call calculates "numerator" / "denominator" and accumulates it into "reference". Its last value is stored into "result"
                    // Alternatively, this could be done using get_hint_field and set_hint_field methods and calculating the operations in Rust,
                    // let (pol_id, airgroupvalue_id) = acc_mul_hint_fields_extended::<F>(
                    //     &sctx,
                    //     &pctx,
                    //     air_instance,
                    //     gprod_hint,
                    //     "reference",
                    //     "result",
                    //     "numerator_air",
                    //     "denominator_air",
                    //     "numerator_direct",
                    //     "denominator_direct",
                    //     HintFieldOptions::default(),
                    //     HintFieldOptions::inverse(),
                    //     HintFieldOptions::default(),
                    //     HintFieldOptions::inverse(),
                    //     false,
                    // );

                    // air_instance.set_commit_calculated(pol_id as usize);
                    // air_instance.set_airgroupvalue_calculated(airgroupvalue_id as usize);

                    let mut gprod = get_hint_field::<F>(&sctx, &pctx, air_instance, gprod_hint, "reference", HintFieldOptions::default());

                    let numerator_air = get_hint_field::<F>(&sctx, &pctx, air_instance, gprod_hint, "numerator_air", HintFieldOptions::default());
                    let denominator_air = get_hint_field::<F>(&sctx, &pctx, air_instance, gprod_hint, "denominator_air", HintFieldOptions::inverse());

                    let numerator_direct = get_hint_field::<F>(&sctx, &pctx, air_instance, gprod_hint, "numerator_direct", HintFieldOptions::default());
                    let denominator_direct = get_hint_field::<F>(&sctx, &pctx, air_instance, gprod_hint, "denominator_direct", HintFieldOptions::inverse());

                    let mut fraq_direct = Vec::new();
                    for i in 0..num_rows {
                        fraq_direct.push(numerator_direct.get(i) * denominator_direct.get(i));
                    }

                    gprod.set(0, numerator_air.get(0) * denominator_air.get(0));
                    for i in 1..num_rows {
                        gprod.set(i, gprod.get(i - 1) * (numerator_air.get(i) * denominator_air.get(i)));
                    }

                    for i in 0..num_rows {
                        gprod.set(i, gprod.get(i) * fraq_direct[i]);
                    }

                    set_hint_field(&sctx, air_instance, gprod_hint as u64, "reference", &gprod);
                    set_hint_field_val(&sctx, air_instance, gprod_hint as u64, "result", gprod.get(num_rows - 1));
                }
            }
        }
    }

    fn end_proof(&self) {
        if self.mode.name == ModeName::Debug {
            let name = Self::MY_NAME;
            let max_values_to_print = self.mode.n_vals;
            let print_to_file = self.mode.print_to_file;
            let debug_data = self.debug_data.as_ref().expect("Debug data missing");
            print_debug_info(name, max_values_to_print, print_to_file, debug_data);
        }
    }
}
