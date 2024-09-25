use proofman_starks_lib_c::verify_global_constraints_c;
use std::ffi::CStr;
use std::cmp;

use std::sync::Arc;

use crate::WitnessLibrary;

use proofman_common::{ExecutionCtx, ProofCtx, ProofOptions, Prover, SetupCtx};

use colored::*;

use std::os::raw::c_void;

pub fn verify_constraints_proof<F>(
    pctx: Arc<ProofCtx<F>>,
    ectx: Arc<ExecutionCtx>,
    sctx: Arc<SetupCtx>,
    provers: Vec<Box<dyn Prover<F>>>,
    mut witness_lib: Box<dyn WitnessLibrary<F>>,
    options: ProofOptions,
) {
    const MY_NAME: &'static str = "ConstraintVerifier";
    let mut proofs: Vec<*mut c_void> = provers.iter().map(|prover| prover.get_proof()).collect();

    log::info!("{}: --> Verifying constraints", MY_NAME);

    witness_lib.debug(pctx.clone(), ectx.clone(), sctx.clone());

    let mut constraints = Vec::new();
    for prover in provers.iter() {
        let constraints_prover_info = prover.verify_constraints(pctx.clone());
        constraints.push(constraints_prover_info);
    }

    let mut valid_constraints = true;
    for (air_instance_index, air_instance) in
        pctx.air_instance_repo.air_instances.read().unwrap().iter().enumerate()
    {
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
                format!("\u{2713} All constraints for Instance #{} were verified", air_instance_index,).bright_green().bold()
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

    let global_constraints_verified = verify_global_constraints_c(
        pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json").to_str().unwrap(),
        pctx.global_info.get_proving_key_path().join("pilout.globalConstraints.bin").to_str().unwrap(),
        public_inputs,
        proofs.as_mut_ptr() as *mut c_void,
        provers.len() as u64,
    );

    log::info!("{}: <-- Checking global constraints", MY_NAME);

    if global_constraints_verified {
        log::info!(
            "{}: ··· {}",
            MY_NAME,
            "\u{2713} All global constraints were successfully verified".bright_green().bold()
        );
    } else {
        log::info!(
            "{}: ··· {}",
            MY_NAME,
            "\u{2717} Not all global constraints were verified".bright_red().bold()
        );
    }

    if valid_constraints && global_constraints_verified {
        log::info!("{}: ··· {}", MY_NAME, "\u{2713} All constraints were verified".bright_green().bold());
    } else {
        log::info!(
            "{}: ··· {}",
            MY_NAME,
            "\u{2717} Not all constraints were verified.".bright_red().bold()
        );
    }
}
