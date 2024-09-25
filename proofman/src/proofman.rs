use libloading::{Library, Symbol};
use log::{info, trace};
use p3_field::Field;
use stark::{StarkBufferAllocator, StarkProver};
use proofman_starks_lib_c::{save_challenges_c, save_publics_c, verify_global_constraints_c};
use std::ffi::CStr;
use std::{cmp, fs};

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use transcript::FFITranscript;

use crate::{WitnessLibrary, WitnessLibInitFn};

use proofman_common::{ConstraintInfo, ExecutionCtx, ProofCtx, Prover, SetupCtx};

use colored::*;

use std::os::raw::c_void;

use proofman_util::{timer_start, timer_stop_and_log};

pub struct ProofMan<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Field + 'static> ProofMan<F> {
    const MY_NAME: &'static str = "ProofMan";

    pub fn generate_proof(
        witness_lib_path: PathBuf,
        rom_path: Option<PathBuf>,
        public_inputs_path: Option<PathBuf>,
        proving_key_path: PathBuf,
        output_dir_path: PathBuf,
        debug_mode: u64,
    ) -> Result<Vec<F>, Box<dyn std::error::Error>> {
        // Check witness_lib path exists
        if !witness_lib_path.exists() {
            return Err(format!("Witness computation dynamic library not found at path: {:?}", witness_lib_path).into());
        }

        // Check rom_path path exists
        if let Some(rom_path) = rom_path.as_ref() {
            if !rom_path.exists() {
                return Err(format!("ROM file not found at path: {:?}", rom_path).into());
            }
        }

        // Check public_inputs_path is a folder
        if let Some(publics_path) = public_inputs_path.as_ref() {
            if !publics_path.exists() {
                return Err(format!("Public inputs file not found at path: {:?}", publics_path).into());
            }
        }

        // Check proving_key_path exists
        if !proving_key_path.exists() {
            return Err(format!("Proving key folder not found at path: {:?}", proving_key_path).into());
        }

        // Check proving_key_path is a folder
        if !proving_key_path.is_dir() {
            return Err(format!("Proving key parameter must be a folder: {:?}", proving_key_path).into());
        }

        if debug_mode == 0 && !output_dir_path.exists() {
            fs::create_dir_all(&output_dir_path)
                .map_err(|err| format!("Failed to create output directory: {:?}", err))?;
        }

        // Load the witness computation dynamic library
        let library = unsafe { Library::new(&witness_lib_path)? };

        let witness_lib: Symbol<WitnessLibInitFn<F>> = unsafe { library.get(b"init_library")? };

        let mut witness_lib = witness_lib(rom_path.clone(), public_inputs_path.clone())?;

        let pctx = ProofCtx::create_ctx(witness_lib.pilout());
        let pctx = Arc::new(pctx);

        let sctx = SetupCtx::new(witness_lib.pilout(), &proving_key_path);
        let sctx = Arc::new(sctx);

        let buffer_allocator: Arc<StarkBufferAllocator> = Arc::new(StarkBufferAllocator::new(proving_key_path.clone()));

        let ectx = ExecutionCtx::builder().with_buffer_allocator(buffer_allocator).build();
        let ectx = Arc::new(ectx);

        Self::initialize_witness(&mut witness_lib, pctx.clone(), ectx.clone(), sctx.clone());

        witness_lib.calculate_witness(1, pctx.clone(), ectx.clone(), sctx.clone());

        Self::print_summary(pctx.clone());

        let mut provers: Vec<Box<dyn Prover<F>>> = Vec::new();
        Self::initialize_provers(sctx.clone(), &proving_key_path, &mut provers, pctx.clone());
        if provers.is_empty() {
            return Err("No instances found".into());
        }
        let mut transcript = provers[0].new_transcript();

        Self::calculate_challenges(0, &mut provers, pctx.clone(), &mut transcript, 0);
        provers[0].add_publics_to_transcript(pctx.clone(), &transcript);

        // Commit stages
        let num_commit_stages = pctx.pilout.num_stages();
        for stage in 1..=num_commit_stages {
            Self::get_challenges(stage, &mut provers, pctx.clone(), &transcript);

            if stage != 1 {
                witness_lib.calculate_witness(stage, pctx.clone(), ectx.clone(), sctx.clone());
            }

            Self::calculate_stage(stage, &mut provers, pctx.clone());

            if debug_mode == 0 {
                Self::commit_stage(stage, &mut provers, pctx.clone());
            }

            if debug_mode == 0 || stage < num_commit_stages {
                Self::calculate_challenges(stage, &mut provers, pctx.clone(), &mut transcript, debug_mode);
            }
        }

        witness_lib.end_proof();

        if debug_mode != 0 {
            let mut proofs: Vec<*mut c_void> = provers.iter().map(|prover| prover.get_proof()).collect();

            log::info!("{}: --> Checking constraints", Self::MY_NAME);

            witness_lib.debug(pctx.clone(), ectx.clone(), sctx.clone());

            let constraints = Self::verify_constraints(&mut provers, pctx.clone());

            let mut valid_constraints = true;
            for (idx, air_instance) in pctx.air_instance_repo.air_instances.read().unwrap().iter().enumerate() {
                let air = pctx.pilout.get_air(air_instance.airgroup_id, air_instance.air_id);

                let mut valid_constraints_prover = true;
                log::info!(
                    "{}:     ► Instance #{}: Air [{}:{}] {}",
                    Self::MY_NAME,
                    idx,
                    air_instance.airgroup_id,
                    air_instance.air_id,
                    air.name().unwrap()
                );
                for constraint in &constraints[idx] {
                    if (debug_mode == 1 && constraint.n_rows == 0) || (debug_mode != 3 && constraint.im_pol) {
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
                            Self::MY_NAME,
                            constraint.stage,
                            valid,
                            line_str.to_str().unwrap()
                        );
                    } else {
                        log::info!(
                            "{}:     · Constraint #{} (stage {}) {} -> {:?}",
                            Self::MY_NAME,
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
                                Self::MY_NAME,
                                row.row,
                                row.value[0]
                            );
                        } else {
                            log::info!(
                                "{}: ···        \u{2717} Failed at row {} with value: [{}, {}, {}]",
                                Self::MY_NAME,
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
                        Self::MY_NAME,
                        format!("\u{2717} Not all constraints for Instance #{} were verified", idx,)
                            .bright_red()
                            .bold()
                    );
                } else {
                    log::info!(
                        "{}:     {}",
                        Self::MY_NAME,
                        format!("\u{2713} All constraints for Instance #{} were verified", idx,).bright_green().bold()
                    );
                }

                if !valid_constraints_prover {
                    valid_constraints = false;
                }
            }

            log::info!("{}: <-- Checking constraints", Self::MY_NAME);

            log::info!("{}: --> Checking global constraints", Self::MY_NAME);

            let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
            let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;

            let global_constraints_verified = verify_global_constraints_c(
                proving_key_path.join("pilout.globalInfo.json").to_str().unwrap(),
                proving_key_path.join("pilout.globalConstraints.bin").to_str().unwrap(),
                public_inputs,
                proofs.as_mut_ptr() as *mut c_void,
                provers.len() as u64,
            );

            log::info!("{}: <-- Checking global constraints", Self::MY_NAME);

            if global_constraints_verified {
                log::info!(
                    "{}: ··· {}",
                    Self::MY_NAME,
                    "\u{2713} All global constraints were successfully verified".bright_green().bold()
                );
            } else {
                log::info!(
                    "{}: ··· {}",
                    Self::MY_NAME,
                    "\u{2717} Not all global constraints were verified".bright_red().bold()
                );
            }

            if valid_constraints && global_constraints_verified {
                log::info!("{}: ··· {}", Self::MY_NAME, "\u{2713} All constraints were verified".bright_green().bold());
            } else {
                log::info!(
                    "{}: ··· {}",
                    Self::MY_NAME,
                    "\u{2717} Not all constraints were verified.".bright_red().bold()
                );
            }

            return Ok(vec![]);
        }

        // Compute Quotient polynomial
        Self::get_challenges(pctx.pilout.num_stages() + 1, &mut provers, pctx.clone(), &transcript);
        Self::calculate_stage(pctx.pilout.num_stages() + 1, &mut provers, pctx.clone());
        Self::commit_stage(pctx.pilout.num_stages() + 1, &mut provers, pctx.clone());
        Self::calculate_challenges(pctx.pilout.num_stages() + 1, &mut provers, pctx.clone(), &mut transcript, 0);

        // Compute openings
        Self::opening_stages(&mut provers, pctx.clone(), &mut transcript);

        let proof = Self::finalize_proof(
            &proving_key_path,
            &mut provers,
            pctx.clone(),
            output_dir_path.to_string_lossy().as_ref(),
        );

        Ok(proof)
    }

    fn initialize_witness(
        witness_lib: &mut Box<dyn WitnessLibrary<F>>,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        witness_lib.start_proof(pctx.clone(), ectx.clone(), sctx.clone());

        log::info!("{}: ··· EXECUTING PROOF", Self::MY_NAME);
        witness_lib.execute(pctx.clone(), ectx, sctx);

        // After the execution print the planned instances
        trace!("{}: --> Air instances: ", Self::MY_NAME);

        let mut group_ids = HashMap::new();

        for air_instance in pctx.air_instance_repo.air_instances.read().unwrap().iter() {
            let group_map = group_ids.entry(air_instance.airgroup_id).or_insert_with(HashMap::new);
            *group_map.entry(air_instance.air_id).or_insert(0) += 1;
        }

        let mut sorted_group_ids: Vec<_> = group_ids.keys().collect();
        sorted_group_ids.sort();

        for &airgroup_id in &sorted_group_ids {
            if let Some(air_map) = group_ids.get(airgroup_id) {
                let mut sorted_air_ids: Vec<_> = air_map.keys().collect();
                sorted_air_ids.sort();

                let air_group = pctx.pilout.get_air_group(*airgroup_id);
                let name = air_group.name().unwrap_or("Unnamed");
                trace!("{}:     + AirGroup [{}] {}", Self::MY_NAME, *airgroup_id, name);

                for &air_id in &sorted_air_ids {
                    if let Some(&count) = air_map.get(air_id) {
                        let air = pctx.pilout.get_air(*airgroup_id, *air_id);
                        let name = air.name().unwrap_or("Unnamed");
                        trace!("{}:       · {} x Air[{}] {}", Self::MY_NAME, count, air.air_id, name);
                    }
                }
            }
        }
    }

    fn initialize_provers(
        sctx: Arc<SetupCtx>,
        proving_key_path: &Path,
        provers: &mut Vec<Box<dyn Prover<F>>>,
        pctx: Arc<ProofCtx<F>>,
    ) {
        timer_start!(INITIALIZING_PROVERS);
        info!("{}: ··· Initializing provers", Self::MY_NAME);

        for (i, air_instance) in pctx.air_instance_repo.air_instances.read().unwrap().iter().enumerate() {
            let prover = Box::new(StarkProver::new(
                sctx.clone(),
                proving_key_path,
                air_instance.airgroup_id,
                air_instance.air_id,
                i,
            ));

            provers.push(prover);
        }

        for prover in provers.iter_mut() {
            prover.build(pctx.clone());
        }

        let mut buff_helper_size = 0_usize;

        for prover in provers.iter_mut() {
            let buff_helper_prover_size = prover.get_buff_helper_size();
            if buff_helper_prover_size > buff_helper_size {
                buff_helper_size = buff_helper_prover_size;
            }
        }

        let buff_helper: Vec<F> = vec![F::zero(); buff_helper_size];

        *pctx.buff_helper.buff_helper.write().unwrap() = buff_helper;
        timer_stop_and_log!(INITIALIZING_PROVERS);
    }

    pub fn verify_constraints(
        provers: &mut [Box<dyn Prover<F>>],
        proof_ctx: Arc<ProofCtx<F>>,
    ) -> Vec<Vec<ConstraintInfo>> {
        let mut invalid_constraints = Vec::new();
        for prover in provers.iter() {
            let invalid_constraints_prover = prover.verify_constraints(proof_ctx.clone());
            invalid_constraints.push(invalid_constraints_prover);
        }
        invalid_constraints
    }

    pub fn calculate_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>], proof_ctx: Arc<ProofCtx<F>>) {
        info!("{}: ··· PROVER STAGE {}", Self::MY_NAME, stage);
        timer_start!(PROVER_STAGE_, stage);

        for prover in provers.iter_mut() {
            prover.calculate_stage(stage, proof_ctx.clone());
        }
        timer_stop_and_log!(PROVER_STAGE_, stage);
    }

    pub fn commit_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>], proof_ctx: Arc<ProofCtx<F>>) {
        info!("{}: Committing stage {}", Self::MY_NAME, stage);

        for (idx, prover) in provers.iter_mut().enumerate() {
            info!("{}: Committing stage {}, for prover {}", Self::MY_NAME, stage, idx);
            prover.commit_stage(stage, proof_ctx.clone());
        }
    }

    fn calculate_challenges(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        proof_ctx: Arc<ProofCtx<F>>,
        transcript: &mut FFITranscript,
        debug_mode: u64,
    ) {
        info!("{}: ··· Calculating challenges", Self::MY_NAME);
        for prover in provers.iter_mut() {
            if debug_mode != 0 {
                let dummy_elements = [F::zero(), F::one(), F::two(), F::neg_one()];
                transcript.add_elements(dummy_elements.as_ptr() as *mut c_void, 4);
            } else {
                prover.add_challenges_to_transcript(stage as u64, proof_ctx.clone(), transcript);
            }
        }
    }

    fn get_challenges(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        proof_ctx: Arc<ProofCtx<F>>,
        transcript: &FFITranscript,
    ) {
        provers[0].get_challenges(stage, proof_ctx, transcript); // Any prover can get the challenges which are common among them
    }

    pub fn opening_stages(
        provers: &mut [Box<dyn Prover<F>>],
        proof_ctx: Arc<ProofCtx<F>>,
        transcript: &mut FFITranscript,
    ) {
        for opening_id in 1..=provers[0].num_opening_stages() {
            Self::get_challenges(
                proof_ctx.pilout.num_stages() + 1 + opening_id,
                provers,
                proof_ctx.clone(),
                transcript,
            );
            for (idx, prover) in provers.iter_mut().enumerate() {
                info!("{}: Opening stage {}, for prover {}", Self::MY_NAME, opening_id, idx);
                prover.opening_stage(opening_id, proof_ctx.clone(), transcript);
            }
            if opening_id < provers[0].num_opening_stages() {
                Self::calculate_challenges(
                    proof_ctx.pilout.num_stages() + 1 + opening_id,
                    provers,
                    proof_ctx.clone(),
                    transcript,
                    0,
                );
            }
        }
    }

    fn finalize_proof(
        proving_key_path: &Path,
        provers: &mut [Box<dyn Prover<F>>],
        proof_ctx: Arc<ProofCtx<F>>,
        output_dir: &str,
    ) -> Vec<F> {
        for (idx, prover) in provers.iter_mut().enumerate() {
            prover.save_proof(idx as u64, output_dir);
        }

        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
        let challenges = (*challenges_guard).as_ptr() as *mut c_void;

        save_publics_c((public_inputs_guard.len() / 8) as u64, public_inputs, output_dir);

        save_challenges_c(challenges, proving_key_path.join("pilout.globalInfo.json").to_str().unwrap(), output_dir);

        vec![]
    }

    fn print_summary(pctx: Arc<ProofCtx<F>>) {
        let air_instances_repo = pctx.air_instance_repo.air_instances.read().unwrap();
        let air_instances_repo = &*air_instances_repo;

        let mut air_instances = HashMap::new();
        for air_instance in air_instances_repo.iter() {
            let air = pctx.pilout.get_air(air_instance.airgroup_id, air_instance.air_id);
            let air_name = air.name().unwrap_or("Unnamed");
            let air_group = pctx.pilout.get_air_group(air_instance.airgroup_id);
            let air_group_name = air_group.name().unwrap_or("Unnamed");
            let air_instance = air_instances.entry(air_group_name).or_insert_with(HashMap::new);
            let air_instance = air_instance.entry(air_name).or_insert(0);
            *air_instance += 1;
        }

        let mut air_groups: Vec<_> = air_instances.keys().collect();
        air_groups.sort();

        info!("{}: >>> PROOF INSTANCES SUMMARY ------------------------", Self::MY_NAME);
        info!("{}:     ► {} Air instances found:", Self::MY_NAME, air_instances_repo.len());
        for air_group in air_groups {
            let air_group_instances = air_instances.get(air_group).unwrap();
            let mut air_names: Vec<_> = air_group_instances.keys().collect();
            air_names.sort();

            info!("{}:       Air Group [{}]", Self::MY_NAME, air_group);
            for air_name in air_names {
                let count = air_group_instances.get(air_name).unwrap();
                log::info!(
                    "{}:       {}",
                    Self::MY_NAME,
                    format!("· {} x Air [{}]", count, air_name).bright_white().bold()
                );
            }
        }
        info!("{}: <<< PROOF INSTANCES SUMMARY ------------------------", Self::MY_NAME);
    }
}
