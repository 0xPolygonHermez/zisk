use libloading::{Library, Symbol};
use log::{debug, info, trace};
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

pub struct ProofMan<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Field + 'static> ProofMan<F> {
    const MY_NAME: &'static str = "ProofMan";

    pub fn generate_proof(
        witness_lib_path: PathBuf,
        rom_path: Option<PathBuf>,
        public_inputs_path: PathBuf,
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
        if !public_inputs_path.exists() {
            return Err(format!("Public inputs file not found at path: {:?}", public_inputs_path).into());
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

        let mut pctx = ProofCtx::create_ctx(witness_lib.pilout());

        let mut provers: Vec<Box<dyn Prover<F>>> = Vec::new();
        
        let sctx = SetupCtx::new(witness_lib.pilout(), &proving_key_path);

        let buffer_allocator: Arc<StarkBufferAllocator> = Arc::new(StarkBufferAllocator::new(proving_key_path.clone()));

        let mut ectx = ExecutionCtx::builder().with_buffer_allocator(buffer_allocator).build();

        Self::initialize_witness(&mut witness_lib, &mut pctx, &mut ectx, &sctx);

        witness_lib.calculate_witness(1, &mut pctx, &ectx, &sctx);

        Self::initialize_provers(&sctx, &proving_key_path, &mut provers, &mut pctx);

        if provers.is_empty() {
            return Err("No instances found".into());
        }
        let mut transcript = provers[0].new_transcript();

        Self::calculate_challenges(0, &mut provers, &mut pctx, &mut transcript, 0);
        provers[0].add_publics_to_transcript(&mut pctx, &transcript);

        // Commit stages
        let num_commit_stages = pctx.pilout.num_stages();
        for stage in 1..=num_commit_stages {
            Self::get_challenges(stage, &mut provers, &mut pctx, &transcript);

            if stage != 1 {
                witness_lib.calculate_witness(stage, &mut pctx, &ectx, &sctx);
            }

            Self::calculate_stage(stage, &mut provers, &mut pctx);

            if debug_mode == 0 {
                Self::commit_stage(stage, &mut provers, &mut pctx);
            }

            if debug_mode == 0 || stage < num_commit_stages  {
                Self::calculate_challenges(stage, &mut provers, &mut pctx, &mut transcript, debug_mode);
            }
        }

        witness_lib.end_proof();

        if debug_mode != 0 {
            let mut proofs: Vec<*mut c_void> = Vec::new();

            for prover in provers.iter_mut() {
                let proof = prover.get_proof();
                proofs.push(proof);
            }

            log::info!("{}: <-- Verifying constraints", Self::MY_NAME);
            
            let constraints = Self::verify_constraints(&mut provers, &mut pctx);

            let valid_constraints = true;
            for (idx, prover) in provers.iter_mut().enumerate() {
                let prover_info = prover.get_prover_info();
                let air_instances = pctx.find_air_instances(prover_info.air_group_id, prover_info.air_id);
                let air_instance_index = air_instances.iter().position(|&x| x == prover_info.prover_idx).unwrap();
                let air = pctx.pilout.get_air(prover_info.air_group_id, prover_info.air_id);
                let mut valid_constraints_prover = true;
                log::debug!("{}: ··· Air {} Instance {}:", Self::MY_NAME, air.name().unwrap(), air_instance_index);
                for constraint in &constraints[idx] {
                    if (debug_mode == 1 && constraint.n_rows == 0) || (debug_mode != 3 && constraint.im_pol) { continue; }
                    let line_str = unsafe { CStr::from_ptr(constraint.line) };
                    let valid =  if constraint.n_rows > 0 { format!("has {} invalid rows", constraint.n_rows).bright_red() } else { "is valid".bright_green() };
                    if constraint.im_pol {
                        log::debug!("{}: ···    Intermediate polynomial (stage {}) {} -> {:?}", Self::MY_NAME, constraint.stage, valid, line_str.to_str().unwrap());
                    } else {
                        log::debug!("{}: ···    Constraint {} (stage {}) {} -> {:?}", Self::MY_NAME, constraint.id, constraint.stage, valid, line_str.to_str().unwrap());
                    }
                    if constraint.n_rows > 0 { valid_constraints_prover = false; }
                    let n_rows = cmp::min(constraint.n_rows, 10);
                    for i in 0..n_rows {
                        let row = constraint.rows[i as usize];
                        if row.dim == 1 {
                            log::debug!("{}: ···        Failed at row {} with value: {}", Self::MY_NAME, row.row, row.value[0]);
                        } else {
                            log::debug!("{}: ···        Failed at row {} with value: [{}, {}, {}]", Self::MY_NAME, row.row, row.value[0], row.value[1], row.value[2]);
                        }
                    }
                    log::debug!("{}: ···   ", Self::MY_NAME);
                }

                if !valid_constraints_prover {
                    log::debug!("{}: ··· {}", Self::MY_NAME, 
                    format!("Not all constraints for instance {} of air {} were verified!",
                    air_instance_index, air.name().unwrap()).bright_yellow().bold());
                } else {                        
                    log::debug!("{}: ··· {}", Self::MY_NAME, 
                        format!("All constraints for instance {} of air {} were verified!",
                        air_instance_index, air.name().unwrap()).bright_cyan().bold());
                }
                log::debug!("{}: ···   ", Self::MY_NAME);
            }

            log::info!("{}: <-- Checking global constraints", Self::MY_NAME);

            let global_constraints_verified = verify_global_constraints_c(&proving_key_path.join("pilout.globalInfo.json").to_str().unwrap(), &proving_key_path.join("pilout.globalConstraints.bin").to_str().unwrap() ,pctx.public_inputs.as_ptr() as *mut c_void, proofs.as_mut_ptr() as *mut c_void, provers.len() as u64);
            
            if !global_constraints_verified {
                log::debug!("{}: ··· {}", Self::MY_NAME, "Not all global constraints were verified.".bright_yellow().bold());          
            } else {
                log::debug!("{}: ··· {}", Self::MY_NAME, "All global constraints were successfully verified.".bright_cyan().bold());
            }

            if valid_constraints && global_constraints_verified {
                log::debug!("{}: ··· {}", Self::MY_NAME, "All constraints were verified!".bright_green().bold());
            } else {
                log::debug!("{}: ··· {}", Self::MY_NAME, "Not all constraints were verified.".bright_red().bold());
            }

            return Ok(vec![]);
        }

        // Compute Quotient polynomial
        Self::get_challenges(pctx.pilout.num_stages() + 1, &mut provers, &mut pctx, &transcript);
        Self::calculate_stage(pctx.pilout.num_stages() + 1, &mut provers, &mut pctx);
        Self::commit_stage(pctx.pilout.num_stages() + 1, &mut provers, &mut pctx);
        Self::calculate_challenges(pctx.pilout.num_stages() + 1, &mut provers, &mut pctx, &mut transcript, 0);

        // Compute openings
        Self::opening_stages(&mut provers, &mut pctx, &mut transcript);

        let proof = Self::finalize_proof(
            &proving_key_path,
            &mut provers,
            &mut pctx,
            output_dir_path.to_string_lossy().as_ref(),
        );

        Ok(proof)
    }

    fn initialize_witness(
        witness_lib: &mut Box<dyn WitnessLibrary<F>>,
        pctx: &mut ProofCtx<F>,
        ectx: &mut ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        witness_lib.start_proof(pctx, ectx, sctx);

        witness_lib.execute(pctx, ectx, sctx);

        // After the execution print the planned instances
        trace!("{}: --> Air instances: ", Self::MY_NAME);

        let mut group_ids = HashMap::new();

        for air_instance in pctx.air_instances.read().unwrap().iter() {
            let group_map = group_ids.entry(air_instance.air_group_id).or_insert_with(HashMap::new);
            *group_map.entry(air_instance.air_id).or_insert(0) += 1;
        }

        let mut sorted_group_ids: Vec<_> = group_ids.keys().collect();
        sorted_group_ids.sort();

        for &air_group_id in &sorted_group_ids {
            if let Some(air_map) = group_ids.get(air_group_id) {
                let mut sorted_air_ids: Vec<_> = air_map.keys().collect();
                sorted_air_ids.sort();

                let air_group = pctx.pilout.get_air_group(*air_group_id);
                let name = air_group.name().unwrap_or("Unnamed");
                trace!("{}:     + AirGroup [{}] {}", Self::MY_NAME, *air_group_id, name);

                for &air_id in &sorted_air_ids {
                    if let Some(&count) = air_map.get(air_id) {
                        let air = pctx.pilout.get_air(*air_group_id, *air_id);
                        let name = air.name().unwrap_or("Unnamed");
                        trace!("{}:       · {} x Air[{}] {}", Self::MY_NAME, count, air.air_id, name);
                    }
                }
            }
        }
    }

    fn initialize_provers(
        sctx: &SetupCtx,
        proving_key_path: &Path,
        provers: &mut Vec<Box<dyn Prover<F>>>,
        pctx: &mut ProofCtx<F>,
    ) {
        info!("{}: Initializing prover and creating buffers", Self::MY_NAME);

        for (prover_idx,air_instance) in pctx.air_instances.write().unwrap().iter_mut().enumerate() {
            debug!(
                "{}: Initializing prover for air instance ({}, {})",
                Self::MY_NAME,
                air_instance.air_group_id,
                air_instance.air_id
            );

            let prover =
                Box::new(StarkProver::new(sctx, proving_key_path, air_instance.air_group_id, air_instance.air_id, prover_idx));

            provers.push(prover);
        }
        for prover in provers.iter_mut() {
            prover.build(pctx);
        }
    }

    pub fn verify_constraints(provers: &mut [Box<dyn Prover<F>>], pctx: &mut ProofCtx<F>) -> Vec<Vec<ConstraintInfo>> {
        let mut invalid_constraints = Vec::new();
        for (_idx, prover) in provers.iter_mut().enumerate() {
            let invalid_constraints_prover = prover.verify_constraints(pctx);
            invalid_constraints.push(invalid_constraints_prover);
        }
        invalid_constraints
    }

    pub fn calculate_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>], pctx: &mut ProofCtx<F>) {
        info!("{}: Calculating stage {}", Self::MY_NAME, stage);
        for (idx, prover) in provers.iter_mut().enumerate() {
            info!("{}: Calculating stage {}, for prover {}", Self::MY_NAME, stage, idx);
            prover.calculate_stage(stage, pctx);
        }
    }

    pub fn commit_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>], pctx: &mut ProofCtx<F>) {
        info!("{}: Committing stage {}", Self::MY_NAME, stage);

        for (idx, prover) in provers.iter_mut().enumerate() {
            info!("{}: Committing stage {}, for prover {}", Self::MY_NAME, stage, idx);
            prover.commit_stage(stage, pctx);
        }
    }

    fn calculate_challenges(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        pctx: &mut ProofCtx<F>,
        transcript: &mut FFITranscript,
        debug_mode: u64,
    ) {
        info!("{}: Calculating challenges for stage {}", Self::MY_NAME, stage);
        for prover in provers.iter_mut() {
            if debug_mode != 0 {
                let dummy_elements = [F::zero(), F::one(), F::two(), F::neg_one()];
                transcript.add_elements(dummy_elements.as_ptr() as *mut c_void, 4);
            } else {
                prover.add_challenges_to_transcript(stage as u64, pctx, transcript);
            }
        }
    }

    fn get_challenges(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        pctx: &mut ProofCtx<F>,
        transcript: &FFITranscript,
    ) {
        info!("{}: Getting challenges for stage {}", Self::MY_NAME, stage);
        provers[0].get_challenges(stage, pctx, transcript); // Any prover can get the challenges which are common among them
    }

    pub fn opening_stages(provers: &mut [Box<dyn Prover<F>>], pctx: &mut ProofCtx<F>, transcript: &mut FFITranscript) {
        for opening_id in 1..=provers[0].num_opening_stages() {
            Self::get_challenges(pctx.pilout.num_stages() + 1 + opening_id, provers, pctx, transcript);
            for (idx, prover) in provers.iter_mut().enumerate() {
                info!("{}: Opening stage {}, for prover {}", Self::MY_NAME, opening_id, idx);
                prover.opening_stage(opening_id, pctx, transcript);
            }
            if opening_id < provers[0].num_opening_stages() {
                Self::calculate_challenges(pctx.pilout.num_stages() + 1 + opening_id, provers, pctx, transcript, 0);
            }
        }
    }

    fn finalize_proof(
        proving_key_path: &Path,
        provers: &mut [Box<dyn Prover<F>>],
        pctx: &mut ProofCtx<F>,
        output_dir: &str,
    ) -> Vec<F> {
        for (idx, prover) in provers.iter_mut().enumerate() {
            prover.save_proof(idx as u64, output_dir);
        }

        save_publics_c(
            (pctx.public_inputs.len() / 8) as u64,
            pctx.public_inputs.as_mut_ptr() as *mut c_void,
            output_dir,
        );

        let mut challenges = pctx.challenges.clone().expect("");
        save_challenges_c(
            challenges.as_mut_ptr() as *mut c_void,
            proving_key_path.join("pilout.globalInfo.json").to_str().unwrap(),
            output_dir,
        );

        vec![]
    }
}
