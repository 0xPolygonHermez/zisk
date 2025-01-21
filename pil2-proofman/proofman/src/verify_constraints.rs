use p3_field::Field;
use proofman_hints::aggregate_airgroupvals;
use proofman_starks_lib_c::{get_n_global_constraints_c, verify_global_constraints_c};
use std::cmp;

use std::sync::Arc;

use proofman_common::{
    get_constraints_lines_str, get_global_constraints_lines_str, skip_prover_instance, GlobalConstraintInfo, ProofCtx,
    Prover, SetupCtx,
};
use std::os::raw::c_void;

use colored::*;

pub fn verify_global_constraints_proof<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    airgroupvalues: Vec<Vec<F>>,
) -> Vec<GlobalConstraintInfo> {
    const MY_NAME: &str = "GlCstVfy";

    log::info!("{}: --> Checking global constraints", MY_NAME);

    let mut airgroup_values_ptrs: Vec<*mut F> = airgroupvalues
        .iter() // Iterate mutably over the inner Vecs
        .map(|inner_vec| inner_vec.as_ptr() as *mut F) // Get a raw pointer to each inner Vec
        .collect();

    let n_global_constraints = get_n_global_constraints_c(sctx.get_global_bin());
    let mut global_constraints_info = vec![GlobalConstraintInfo::default(); n_global_constraints as usize];

    if !pctx.options.debug_info.debug_global_instances.is_empty() {
        global_constraints_info.iter_mut().for_each(|constraint| constraint.skip = true);
        for constraint_id in &pctx.options.debug_info.debug_global_instances {
            global_constraints_info[*constraint_id].skip = false;
        }
    }
    verify_global_constraints_c(
        sctx.get_global_info_file().as_str(),
        sctx.get_global_bin(),
        pctx.get_publics_ptr(),
        pctx.get_challenges_ptr(),
        pctx.get_proof_values_ptr(),
        airgroup_values_ptrs.as_mut_ptr() as *mut *mut u8,
        global_constraints_info.as_mut_ptr() as *mut c_void,
    );

    global_constraints_info
}

pub fn verify_constraints_proof<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    provers: &mut [Box<dyn Prover<F>>],
) -> Result<(), Box<dyn std::error::Error>> {
    const MY_NAME: &str = "CstrVrfy";

    log::info!("{}: --> Checking constraints", MY_NAME);

    let mut constraints = Vec::new();
    for prover in provers.iter() {
        let constraints_prover_info = prover.verify_constraints(sctx.clone(), pctx.clone());
        constraints.push(constraints_prover_info);
    }

    let mut valid_constraints = true;

    let instances = pctx.dctx_get_instances();
    let my_instances = pctx.dctx_get_my_instances();

    for instance_id in my_instances.iter() {
        let (airgroup_id, air_id) = instances[*instance_id];
        let air_name = &pctx.global_info.airs[airgroup_id][air_id].name;
        let air_instance_id = pctx.dctx_find_air_instance_id(*instance_id);
        let (skip, _) = skip_prover_instance(pctx.options.clone(), airgroup_id, air_id, air_instance_id);
        if skip {
            log::info!(
                "{}",
                format!(
                    "{}: ··· \u{2713} Skipping Instance #{} of {} [{}:{}]",
                    MY_NAME, air_instance_id, air_name, airgroup_id, air_id
                )
                .bright_yellow()
                .bold()
            );
        };
    }

    for (idx, prover) in provers.iter().enumerate() {
        let prover_info = prover.get_prover_info();
        let (airgroup_id, air_id, air_instance_id) =
            (prover_info.airgroup_id, prover_info.air_id, prover_info.air_instance_id);

        let air_name = &pctx.global_info.airs[airgroup_id][air_id].name;

        let constraints_lines = get_constraints_lines_str(sctx.clone(), airgroup_id, air_id);

        let mut valid_constraints_prover = true;
        let skipping = "is skipped".bright_yellow();

        log::info!("{}:     ► Instance #{} of {} [{}:{}]", MY_NAME, air_instance_id, air_name, airgroup_id, air_id,);
        for constraint in &constraints[idx] {
            if constraint.skip {
                log::debug!(
                    "{}:     · Constraint #{} (stage {}) {} -> {}",
                    MY_NAME,
                    constraint.id,
                    constraint.stage,
                    skipping,
                    constraints_lines[constraint.id as usize]
                );
                continue;
            }
            let valid = if constraint.n_rows > 0 {
                format!("has {} invalid rows", constraint.n_rows).bright_red()
            } else {
                "is valid".bright_green()
            };
            if constraint.im_pol {
                log::trace!(
                    "{}: ···    Intermediate polynomial (stage {}) {} -> {}",
                    MY_NAME,
                    constraint.stage,
                    valid,
                    constraints_lines[constraint.id as usize]
                );
            } else if constraint.n_rows == 0 {
                log::debug!(
                    "{}:     · Constraint #{} (stage {}) {} -> {}",
                    MY_NAME,
                    constraint.id,
                    constraint.stage,
                    valid,
                    constraints_lines[constraint.id as usize]
                );
            } else {
                log::info!(
                    "{}:     · Constraint #{} (stage {}) {} -> {}",
                    MY_NAME,
                    constraint.id,
                    constraint.stage,
                    valid,
                    constraints_lines[constraint.id as usize]
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
                format!("\u{2717} Not all constraints for Instance #{} of {} were verified", air_instance_id, air_name)
                    .bright_red()
                    .bold()
            );
        } else {
            log::info!(
                "{}:     {}",
                MY_NAME,
                format!("\u{2713} All constraints for Instance #{} of {} were verified", air_instance_id, air_name)
                    .bright_green()
                    .bold()
            );
        }

        if !valid_constraints_prover {
            valid_constraints = false;
        }
    }

    let airgroupvalues_u64 = aggregate_airgroupvals(pctx.clone());

    let check_global_constraints = pctx.options.debug_info.debug_instances.is_empty()
        || !pctx.options.debug_info.debug_global_instances.is_empty();

    let airgroupvalues = pctx.dctx_distribute_airgroupvalues(airgroupvalues_u64);
    if pctx.dctx_get_rank() == 0 && check_global_constraints {
        let global_constraints = verify_global_constraints_proof(pctx.clone(), sctx.clone(), airgroupvalues);
        let mut valid_global_constraints = true;

        let global_constraints_lines = get_global_constraints_lines_str(sctx.clone());

        for idx in 0..global_constraints.len() {
            let constraint = global_constraints[idx];
            let line_str = &global_constraints_lines[idx];

            if constraint.skip {
                log::debug!(
                    "{}:     · Global Constraint #{} {} -> {}",
                    MY_NAME,
                    idx,
                    "is skipped".bright_yellow(),
                    line_str,
                );
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

        if valid_constraints && valid_global_constraints {
            log::info!("{}: ··· {}", MY_NAME, "\u{2713} All constraints were verified".bright_green().bold());
            Ok(())
        } else {
            log::info!("{}: ··· {}", MY_NAME, "\u{2717} Not all constraints were verified.".bright_red().bold());
            Err(Box::new(std::io::Error::new(
                // <-- Return a boxed error
                std::io::ErrorKind::Other,
                format!("{}: Not all constraints were verified.", MY_NAME),
            )))
        }
    } else {
        if check_global_constraints {
            log::info!(
                "{}: ··· {}",
                MY_NAME,
                "\u{2713} Skipping global constraints verification".bright_yellow().bold()
            );
        }
        if valid_constraints {
            log::info!("{}: ··· {}", MY_NAME, "\u{2713} All constraints were verified".bright_green().bold());
            Ok(())
        } else {
            log::info!("{}: ··· {}", MY_NAME, "\u{2717} Not all constraints were verified.".bright_red().bold());
            Err(Box::new(std::io::Error::new(
                // <-- Return a boxed error
                std::io::ErrorKind::Other,
                format!("{}: Not all constraints were verified.", MY_NAME),
            )))
        }
    }
}
