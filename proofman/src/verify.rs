use p3_field::Field;

use proofman_starks_lib_c::{stark_info_new_c, expressions_bin_new_c, stark_verify_c};

use colored::*;

use std::sync::Arc;

use proofman_common::{ProofCtx, ProofType, Prover, SetupCtx, get_global_constraints_lines_str};

use proofman_hints::aggregate_airgroupvals;
use proofman_util::{timer_start_info, timer_stop_and_log_info};

use std::os::raw::c_void;

use crate::verify_global_constraints_proof;

pub fn verify_proof<F: Field>(
    p_proof: *mut c_void,
    stark_info_path: String,
    expressions_bin_path: String,
    verkey_path: String,
    publics: Option<Vec<F>>,
    proof_values: Option<Vec<F>>,
    challenges: Option<Vec<F>>,
) -> bool {
    let p_stark_info = stark_info_new_c(stark_info_path.as_str(), true);
    let p_expressions_bin = expressions_bin_new_c(expressions_bin_path.as_str(), false, true);

    let proof_challenges_ptr = match challenges {
        Some(ref challenges) => challenges.as_ptr() as *mut u8,
        None => std::ptr::null_mut(),
    };

    let publics_ptr = match publics {
        Some(ref publics) => publics.as_ptr() as *mut u8,
        None => std::ptr::null_mut(),
    };

    let proof_values_ptr = match proof_values {
        Some(ref proof_values) => proof_values.as_ptr() as *mut u8,
        None => std::ptr::null_mut(),
    };

    stark_verify_c(
        &verkey_path,
        p_proof,
        p_stark_info,
        p_expressions_bin,
        publics_ptr,
        proof_values_ptr,
        proof_challenges_ptr,
    )
}

pub fn verify_basic_proofs<F: Field>(
    provers: &mut [Box<dyn Prover<F>>],
    proves: Vec<*mut c_void>,
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
) -> bool {
    const MY_NAME: &str = "Verify  ";
    timer_start_info!(VERIFYING_BASIC_PROOFS);
    let mut is_valid = true;

    for (idx, prover) in provers.iter().enumerate() {
        let prover_info = prover.get_prover_info();

        let setup_path =
            pctx.global_info.get_air_setup_path(prover_info.airgroup_id, prover_info.air_id, &ProofType::Basic);

        let steps_fri: Vec<usize> = pctx.global_info.steps_fri.iter().map(|step| step.n_bits).collect();
        let proof_challenges = prover.get_proof_challenges(steps_fri, pctx.get_challenges().to_vec());

        let stark_info_path = setup_path.display().to_string() + ".starkinfo.json";
        let expressions_bin_path = setup_path.display().to_string() + ".verifier.bin";
        let verkey_path = setup_path.display().to_string() + ".verkey.json";

        let is_valid_proof = verify_proof(
            proves[idx],
            stark_info_path,
            expressions_bin_path,
            verkey_path,
            Some(pctx.get_publics().clone()),
            Some(pctx.get_proof_values().clone()),
            Some(proof_challenges),
        );

        let air_name = &pctx.global_info.airs[prover_info.airgroup_id][prover_info.air_id].name;

        if !is_valid_proof {
            is_valid = false;
            log::info!(
                "{}: ··· {}",
                MY_NAME,
                format!("\u{2717} Proof of {}: Instance #{} was not verified", air_name, prover_info.air_instance_id,)
                    .bright_red()
                    .bold()
            );
        } else {
            log::info!(
                "{}:     {}",
                MY_NAME,
                format!("\u{2713} Proof of {}: Instance #{} was verified", air_name, prover_info.air_instance_id,)
                    .bright_green()
                    .bold()
            );
        }
    }

    let check_global_constraints = pctx.options.debug_info.debug_instances.is_empty()
        || !pctx.options.debug_info.debug_global_instances.is_empty();

    let airgroupvalues_u64 = aggregate_airgroupvals(pctx.clone());

    let airgroupvalues = pctx.dctx_distribute_airgroupvalues(airgroupvalues_u64);
    if pctx.dctx_get_rank() == 0 && check_global_constraints {
        let global_constraints = verify_global_constraints_proof(pctx.clone(), sctx.clone(), airgroupvalues);
        let mut valid_global_constraints = true;

        let global_constraints_lines = get_global_constraints_lines_str(sctx.clone());

        for idx in 0..global_constraints.len() {
            let constraint = global_constraints[idx];
            let line_str = &global_constraints_lines[idx];

            if constraint.skip {
                log::debug!("{}:     · Skipping Global Constraint #{} -> {}", MY_NAME, idx, line_str,);
                continue;
            }

            let valid = if !constraint.valid { "is invalid".bright_red() } else { "is valid".bright_green() };
            if constraint.valid {
                log::debug!("{}:     · Global Constraint #{} {} -> {}", MY_NAME, constraint.id, valid, line_str);
            } else {
                log::info!("{}:     · Global Constraint #{} {} -> {}", MY_NAME, constraint.id, valid, line_str);
            }
            if !constraint.valid {
                valid_global_constraints = false;
                if constraint.dim == 1 {
                    log::info!("{}: ···        \u{2717} Failed with value: {}", MY_NAME, constraint.value[0]);
                } else {
                    log::info!(
                        "{}: ···        \u{2717} Failed with value: [{}, {}, {}]",
                        MY_NAME,
                        constraint.value[0],
                        constraint.value[1],
                        constraint.value[2]
                    );
                }
            }
        }

        if valid_global_constraints {
            log::info!(
                "{}: ··· {}",
                MY_NAME,
                "\u{2713} All global constraints were successfully verified".bright_green().bold()
            );
        } else {
            log::info!("{}: ··· {}", MY_NAME, "\u{2717} Not all global constraints were verified".bright_red().bold());
        }

        if is_valid && valid_global_constraints {
            log::info!("{}: ··· {}", MY_NAME, "\u{2713} All proofs were verified".bright_green().bold());
        } else {
            log::info!("{}: ··· {}", MY_NAME, "\u{2717} Not all proofs were verified.".bright_red().bold());
            is_valid = false;
        }
    } else if check_global_constraints {
        log::info!("{}: ··· {}", MY_NAME, "\u{2713} Skipping global constraints verification".bright_yellow().bold());
    } else if is_valid {
        log::info!("{}: ··· {}", MY_NAME, "\u{2713} All proofs were verified".bright_green().bold());
    } else {
        log::info!("{}: ··· {}", MY_NAME, "\u{2717} Not all proofs were verified.".bright_red().bold());
    }

    timer_stop_and_log_info!(VERIFYING_BASIC_PROOFS);
    is_valid
}
