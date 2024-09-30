use p3_field::Field;
use proofman_starks_lib_c::verify_global_constraints_c;
use std::ffi::CStr;
use std::cmp;

use std::sync::Arc;

use crate::WitnessLibrary;

use proofman_common::{ExecutionCtx, ExtensionField, ProofCtx, ProofOptions, Prover, SetupCtx};

use colored::*;

use std::os::raw::c_void;

pub fn verify_constraints_proof<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    ectx: Arc<ExecutionCtx>,
    sctx: Arc<SetupCtx>,
    provers: Vec<Box<dyn Prover<F>>>,
    mut witness_lib: Box<dyn WitnessLibrary<F>>,
    options: ProofOptions,
) {
    const MY_NAME: &str = "ConstraintVerifier";
    const FIELD_EXTENSION: usize = 3;

    log::info!("{}: --> Verifying constraints", MY_NAME);

    witness_lib.debug(pctx.clone(), ectx.clone(), sctx.clone());

    let mut constraints = Vec::new();
    for prover in provers.iter() {
        let constraints_prover_info = prover.verify_constraints(pctx.clone());
        constraints.push(constraints_prover_info);
    }

    let mut valid_constraints = true;
    for (air_instance_index, air_instance) in pctx.air_instance_repo.air_instances.read().unwrap().iter().enumerate() {
        let air_name = &pctx.global_info.airs[air_instance.airgroup_id][air_instance.air_id].name;
        let mut valid_constraints_prover = true;
        log::info!(
            "{}:     ► Instance #{}: Air [{}:{}] {}",
            MY_NAME,
            air_instance_index,
            air_instance.airgroup_id,
            air_instance.air_id,
            air_name,
        );
        for constraint in &constraints[air_instance_index] {
            if (options.debug_mode == 1 && constraint.n_rows == 0) || (options.debug_mode != 3 && constraint.im_pol) {
                continue;
            }
            let line_str = unsafe { CStr::from_ptr(constraint.line) };
            let valid = if constraint.n_rows > 0 {
                format!("has {} invalid rows", constraint.n_rows).bright_red()
            } else {
                "is valid".bright_green()
            };
            if constraint.im_pol {
                log::info!(
                    "{}: ···    Intermediate polynomial (stage {}) {} -> {:?}",
                    MY_NAME,
                    constraint.stage,
                    valid,
                    line_str.to_str().unwrap()
                );
            } else {
                log::info!(
                    "{}:     · Constraint #{} (stage {}) {} -> {:?}",
                    MY_NAME,
                    constraint.id,
                    constraint.stage,
                    valid,
                    line_str.to_str().unwrap()
                );
            }
            if constraint.n_rows > 0 {
                valid_constraints_prover = false;
            }
            let n_rows = cmp::min(constraint.n_rows, 10);
            for i in 0..n_rows {
                let row = constraint.rows[i as usize];
                if row.dim == 1 {
                    log::info!(
                        "{}: ···        \u{2717} Failed at row {} with value: {}",
                        MY_NAME,
                        row.row,
                        row.value[0]
                    );
                } else {
                    log::info!(
                        "{}: ···        \u{2717} Failed at row {} with value: [{}, {}, {}]",
                        MY_NAME,
                        row.row,
                        row.value[0],
                        row.value[1],
                        row.value[2]
                    );
                }
            }
        }

        if !valid_constraints_prover {
            log::info!(
                "{}: ··· {}",
                MY_NAME,
                format!("\u{2717} Not all constraints for Instance #{} were verified", air_instance_index,)
                    .bright_red()
                    .bold()
            );
        } else {
            log::info!(
                "{}:     {}",
                MY_NAME,
                format!("\u{2713} All constraints for Instance #{} were verified", air_instance_index,)
                    .bright_green()
                    .bold()
            );
        }

        if !valid_constraints_prover {
            valid_constraints = false;
        }
    }

    log::info!("{}: <-- Checking constraints", MY_NAME);

    log::info!("{}: --> Checking global constraints", MY_NAME);

    let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
    let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;

    let mut airgroupvalues: Vec<Vec<F>> = Vec::new();
    for agg_types in pctx.global_info.agg_types.iter() {
        let mut values = vec![F::zero(); agg_types.len() * FIELD_EXTENSION];
        for (idx, agg_type) in agg_types.iter().enumerate() {
            if agg_type.agg_type == 1 {
                values[idx * FIELD_EXTENSION] = F::one();
            }
        }
        airgroupvalues.push(values);
    }

    for prover in provers.iter() {
        let prover_info = prover.get_prover_info();
        let airgroup_vals =
            &mut pctx.air_instance_repo.air_instances.write().unwrap()[prover_info.prover_idx].subproof_values;
        for (idx, agg_type) in pctx.global_info.agg_types[prover_info.airgroup_id].iter().enumerate() {
            let mut acc = ExtensionField {
                value: [
                    airgroupvalues[prover_info.airgroup_id][idx * FIELD_EXTENSION],
                    airgroupvalues[prover_info.airgroup_id][idx * FIELD_EXTENSION + 1],
                    airgroupvalues[prover_info.airgroup_id][idx * FIELD_EXTENSION + 2],
                ],
            };
            let instance_airgroup_val = ExtensionField {
                value: [
                    airgroup_vals[idx * FIELD_EXTENSION],
                    airgroup_vals[idx * FIELD_EXTENSION + 1],
                    airgroup_vals[idx * FIELD_EXTENSION + 2],
                ],
            };
            if agg_type.agg_type == 0 {
                acc += instance_airgroup_val;
            } else {
                acc *= instance_airgroup_val;
            }
            airgroupvalues[prover_info.airgroup_id][idx * FIELD_EXTENSION] = acc.value[0];
            airgroupvalues[prover_info.airgroup_id][idx * FIELD_EXTENSION + 1] = acc.value[1];
            airgroupvalues[prover_info.airgroup_id][idx * FIELD_EXTENSION + 2] = acc.value[2];
        }
    }

    let mut airgroup_values_ptrs: Vec<*mut F> = airgroupvalues
        .iter_mut() // Iterate mutably over the inner Vecs
        .map(|inner_vec| inner_vec.as_mut_ptr()) // Get a raw pointer to each inner Vec
        .collect();

    let global_constraints_verified = verify_global_constraints_c(
        pctx.global_info.get_proving_key_path().join("pilout.globalConstraints.bin").to_str().unwrap(),
        public_inputs,
        airgroup_values_ptrs.as_mut_ptr() as *mut *mut c_void,
    );

    log::info!("{}: <-- Checking global constraints", MY_NAME);

    if global_constraints_verified {
        log::info!(
            "{}: ··· {}",
            MY_NAME,
            "\u{2713} All global constraints were successfully verified".bright_green().bold()
        );
    } else {
        log::info!("{}: ··· {}", MY_NAME, "\u{2717} Not all global constraints were verified".bright_red().bold());
    }

    if valid_constraints && global_constraints_verified {
        log::info!("{}: ··· {}", MY_NAME, "\u{2713} All constraints were verified".bright_green().bold());
    } else {
        log::info!("{}: ··· {}", MY_NAME, "\u{2717} Not all constraints were verified.".bright_red().bold());
    }
}
