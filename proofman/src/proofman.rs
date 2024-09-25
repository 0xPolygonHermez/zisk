use libloading::{Library, Symbol};
use log::{info, trace};
use p3_field::Field;
use stark::{StarkBufferAllocator, StarkProver};
use proofman_starks_lib_c::{save_challenges_c, save_publics_c, verify_global_constraints_c};
use std::ffi::{CString, CStr};
use std::os::raw::c_char;
use std::{cmp, fs};
use proofman_starks_lib_c::*;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use transcript::FFITranscript;

use crate::{WitnessLibrary, WitnessLibInitFn};

use proofman_common::{ConstraintInfo, ExecutionCtx, ProofCtx, ProofType, Prover, SetupCtx};

use colored::*;

use std::os::raw::c_void;

use proofman_util::{timer_start, timer_stop_and_log};

pub struct ProofMan<F> {
    _phantom: std::marker::PhantomData<F>,
}

type GetCommitedPolsFunc = unsafe extern "C" fn(
    p_address: *mut c_void,
    p_publics: *mut c_void,
    zkin: *mut c_void,
    n: u64,
    n_publics: u64,
    offset_cm1: u64,
    dat_file: *const c_char,
    exec_file: *const c_char,
);

pub struct ProofOptions {
    pub debug_mode: u64,
    pub aggregation: bool,
    pub save_proofs: bool,
}

impl ProofOptions {
    pub fn new(debug_mode: u64, aggregation: bool, save_proofs: bool) -> Self {
        Self { debug_mode, aggregation, save_proofs }
    }
}

impl<F: Field + 'static> ProofMan<F> {
    const MY_NAME: &'static str = "ProofMan";

    pub fn generate_proof(
        witness_lib_path: PathBuf,
        rom_path: Option<PathBuf>,
        public_inputs_path: Option<PathBuf>,
        proving_key_path: PathBuf,
        output_dir_path: PathBuf,
        options: ProofOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

        if options.debug_mode == 0 && !output_dir_path.exists() {
            fs::create_dir_all(&output_dir_path)
                .map_err(|err| format!("Failed to create output directory: {:?}", err))?;
        }

        // Load the witness computation dynamic library
        let library = unsafe { Library::new(&witness_lib_path)? };

        let witness_lib: Symbol<WitnessLibInitFn<F>> = unsafe { library.get(b"init_library")? };

        let mut witness_lib = witness_lib(rom_path.clone(), public_inputs_path.clone())?;

        let pctx = Arc::new(ProofCtx::create_ctx(proving_key_path.clone()));

        let sctx = Arc::new(SetupCtx::new(&pctx.global_info, &ProofType::Basic));

        let buffer_allocator: Arc<StarkBufferAllocator> = Arc::new(StarkBufferAllocator::new(proving_key_path.clone()));

        let ectx = ExecutionCtx::builder().with_buffer_allocator(buffer_allocator).build();
        let ectx = Arc::new(ectx);

        Self::initialize_witness(&mut witness_lib, pctx.clone(), ectx.clone(), sctx.clone());

        witness_lib.calculate_witness(1, pctx.clone(), ectx.clone(), sctx.clone());

        Self::print_summary(pctx.clone());

        let mut provers: Vec<Box<dyn Prover<F>>> = Vec::new();
        Self::initialize_provers(sctx.clone(), &mut provers, pctx.clone());
        if provers.is_empty() {
            return Err("No instances found".into());
        }
        let mut transcript = provers[0].new_transcript();

        Self::calculate_challenges(0, &mut provers, pctx.clone(), &mut transcript, 0);
        provers[0].add_publics_to_transcript(pctx.clone(), &transcript);

        // Commit stages
        let num_commit_stages = pctx.global_info.n_challenges.len() as u32;
        for stage in 1..=num_commit_stages {
            Self::get_challenges(stage, &mut provers, pctx.clone(), &transcript);

            if stage != 1 {
                witness_lib.calculate_witness(stage, pctx.clone(), ectx.clone(), sctx.clone());
            }

            Self::calculate_stage(stage, &mut provers, pctx.clone());

            if options.debug_mode == 0 {
                Self::commit_stage(stage, &mut provers, pctx.clone());
            }

            if options.debug_mode == 0 || stage < num_commit_stages {
                Self::calculate_challenges(stage, &mut provers, pctx.clone(), &mut transcript, options.debug_mode);
            }
        }

        witness_lib.end_proof();

        if options.debug_mode != 0 {
            let mut proofs: Vec<*mut c_void> = provers.iter().map(|prover| prover.get_proof()).collect();

            log::info!("{}: --> Checking constraints", Self::MY_NAME);

            witness_lib.debug(pctx.clone(), ectx.clone(), sctx.clone());

            let constraints = Self::verify_constraints(&mut provers, pctx.clone());

            let mut valid_constraints = true;
            for (air_instance_index, air_instance) in
                pctx.air_instance_repo.air_instances.read().unwrap().iter().enumerate()
            {
                let air_name = &pctx.global_info.airs[air_instance.airgroup_id][air_instance.air_id].name;
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

            return Ok(());
        }

        // Compute Quotient polynomial
        Self::get_challenges(num_commit_stages + 1, &mut provers, pctx.clone(), &transcript);
        Self::calculate_stage(num_commit_stages + 1, &mut provers, pctx.clone());
        Self::commit_stage(num_commit_stages + 1, &mut provers, pctx.clone());
        Self::calculate_challenges(num_commit_stages + 1, &mut provers, pctx.clone(), &mut transcript, 0);

        // Compute openings
        Self::opening_stages(&mut provers, pctx.clone(), sctx.clone(), &mut transcript);

        //Generate proves_out
        let proves_out = Self::finalize_proof(
            &mut provers,
            pctx.clone(),
            output_dir_path.to_string_lossy().as_ref(),
            options.aggregation,
            options.save_proofs,
        );

        if !options.aggregation {
            return Ok(());
        }

        let comp_proofs = Self::generate_recursion_proof(
            &pctx,
            &proves_out,
            &ProofType::Compressor,
            output_dir_path.clone(),
            options.save_proofs,
        )?;
        println!("Compressor proofs generated successfully");

        let recursive1_proofs = Self::generate_recursion_proof(
            &pctx,
            &comp_proofs,
            &ProofType::Recursive1,
            output_dir_path.clone(),
            options.save_proofs,
        )?;
        println!("Recursive1 proofs generated successfully");

        let recursive2_proofs = Self::generate_recursion_proof(
            &pctx,
            &recursive1_proofs,
            &ProofType::Recursive2,
            output_dir_path.clone(),
            options.save_proofs,
        )?;
        println!("Recursive2 proofs generated successfully");

        let _final_proof = Self::generate_recursion_proof(
            &pctx,
            &recursive2_proofs,
            &ProofType::Final,
            output_dir_path.clone(),
            true,
        )?;
        println!("Final proof generated successfully");

        Ok(())
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

                let airgroup_name = &pctx.global_info.subproofs[*airgroup_id];
                trace!("{}:     + AirGroup [{}] {}", Self::MY_NAME, airgroup_id, airgroup_name);

                for &air_id in &sorted_air_ids {
                    if let Some(&count) = air_map.get(air_id) {
                        let air_name = &pctx.global_info.airs[*airgroup_id][*air_id].name;
                        trace!("{}:       · {} x Air[{}] {}", Self::MY_NAME, count, air_id, air_name);
                    }
                }
            }
        }
    }

    fn initialize_provers(sctx: Arc<SetupCtx>, provers: &mut Vec<Box<dyn Prover<F>>>, pctx: Arc<ProofCtx<F>>) {
        info!("{}: Initializing prover and creating buffers", Self::MY_NAME);

        timer_start!(INITIALIZING_PROVERS);
        for (prover_idx, air_instance) in pctx.air_instance_repo.air_instances.read().unwrap().iter().enumerate() {
            log::debug!(
                "{}: Initializing prover for air instance ({}, {})",
                Self::MY_NAME,
                air_instance.airgroup_id,
                air_instance.air_id
            );

            let prover = Box::new(StarkProver::new(
                sctx.clone(),
                pctx.clone(),
                air_instance.airgroup_id,
                air_instance.air_id,
                prover_idx,
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
        info!("{}: Calculating challenges for stage {}", Self::MY_NAME, stage);
        let airgroups = proof_ctx.global_info.subproofs.clone();
        for (airgroup_id, _airgroup) in airgroups.iter().enumerate() {
            let airgroup_instances = proof_ctx.air_instance_repo.find_airgroup_instances(airgroup_id);
            for prover_idx in airgroup_instances.iter() {
                if debug_mode != 0 {
                    let dummy_elements = [F::zero(), F::one(), F::two(), F::neg_one()];
                    transcript.add_elements(dummy_elements.as_ptr() as *mut c_void, 4);
                } else {
                    provers[*prover_idx].add_challenges_to_transcript(stage as u64, proof_ctx.clone(), transcript);
                }
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
        setup_ctx: Arc<SetupCtx>,
        transcript: &mut FFITranscript,
    ) {
        let setup_airs = setup_ctx.get_setup_airs();

        let num_commit_stages = proof_ctx.global_info.n_challenges.len() as u32;

        // Calculate evals
        Self::get_challenges(num_commit_stages + 2, provers, proof_ctx.clone(), transcript);
        for (airgroup_id, airgroup) in setup_airs.iter().enumerate() {
            for air_id in airgroup.iter() {
                let air_instances_idx: Vec<usize> =
                    proof_ctx.air_instance_repo.find_air_instances(airgroup_id, *air_id);
                if !air_instances_idx.is_empty() {
                    provers[air_instances_idx[0]].calculate_lev(proof_ctx.clone());

                    for idx in air_instances_idx {
                        info!("{}: Opening stage {}, for prover {}", Self::MY_NAME, 1, idx);
                        provers[idx].opening_stage(1, proof_ctx.clone());
                    }
                }
            }
        }
        Self::calculate_challenges(num_commit_stages + 2, provers, proof_ctx.clone(), transcript, 0);

        // Calculate fri polynomial
        Self::get_challenges(num_commit_stages + 3, provers, proof_ctx.clone(), transcript);
        for (airgroup_id, airgroup) in setup_airs.iter().enumerate() {
            for air_id in airgroup.iter() {
                let air_instances_idx: Vec<usize> =
                    proof_ctx.air_instance_repo.find_air_instances(airgroup_id, *air_id);
                if !air_instances_idx.is_empty() {
                    provers[air_instances_idx[0]].calculate_xdivxsub(proof_ctx.clone());

                    for idx in air_instances_idx {
                        info!("{}: Opening stage {}, for prover {}", Self::MY_NAME, 2, idx);
                        provers[idx].opening_stage(2, proof_ctx.clone());
                    }
                }
            }
        }
        Self::calculate_challenges(num_commit_stages + 3, provers, proof_ctx.clone(), transcript, 0);

        // FRI Steps
        for opening_id in 3..=provers[0].num_opening_stages() {
            Self::get_challenges(num_commit_stages + 1 + opening_id, provers, proof_ctx.clone(), transcript);
            for (idx, prover) in provers.iter_mut().enumerate() {
                info!("{}: Computing FRI step {} for prover {}", Self::MY_NAME, opening_id - 3, idx);
                prover.opening_stage(opening_id, proof_ctx.clone());
            }
            if opening_id < provers[0].num_opening_stages() {
                Self::calculate_challenges(
                    num_commit_stages + 1 + opening_id,
                    provers,
                    proof_ctx.clone(),
                    transcript,
                    0,
                );
            }
        }
    }

    fn finalize_proof(
        provers: &mut [Box<dyn Prover<F>>],
        proof_ctx: Arc<ProofCtx<F>>,
        output_dir: &str,
        aggregation: bool,
        save_proofs: bool,
    ) -> Vec<*mut c_void> {
        let mut proves = Vec::new();
        for prover in provers.iter_mut() {
            proves.push(prover.save_proof(proof_ctx.clone(), output_dir, save_proofs));
        }

        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let n_publics = proof_ctx.global_info.n_publics as u64;
        let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
        let challenges = (*challenges_guard).as_ptr() as *mut c_void;

        let global_info_path = proof_ctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
        let global_info_file: &str = global_info_path.to_str().unwrap();

        if aggregation || save_proofs {
            save_publics_c(n_publics, public_inputs, output_dir);
            save_challenges_c(challenges, global_info_file, output_dir);
        }

        proves
    }

    //
    // Recursion prove
    //

    pub fn generate_recursion_proof(
        pctx: &ProofCtx<F>,
        proves: &[*mut c_void],
        proof_type: &ProofType,
        output_dir_path: PathBuf,
        save_proof: bool,
    ) -> Result<Vec<*mut c_void>, Box<dyn std::error::Error>> {
        //Create setup contexts
        let mut proves_out: Vec<*mut c_void> = Vec::new();

        let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
        let global_info_file: &str = global_info_path.to_str().unwrap();

        // Run proves
        match *proof_type {
            ProofType::Compressor | ProofType::Recursive1 => {
                let sctx = SetupCtx::new(&pctx.global_info, proof_type);

                for (prover_idx, air_instance) in
                    pctx.air_instance_repo.air_instances.write().unwrap().iter_mut().enumerate()
                {
                    let air_setup_folder =
                        pctx.global_info.get_air_setup_path(air_instance.airgroup_id, air_instance.air_id, proof_type);
                    trace!("{}   : ··· Setup AIR folder: {:?}", Self::MY_NAME, air_setup_folder);

                    // Check path exists and is a folder
                    if !air_setup_folder.exists() {
                        panic!("Setup AIR folder not found at path: {:?}", air_setup_folder);
                    }
                    if !air_setup_folder.is_dir() {
                        panic!("Setup AIR path is not a folder: {:?}", air_setup_folder);
                    }

                    if *proof_type == ProofType::Compressor
                        && !pctx.global_info.get_air_has_compressor(air_instance.airgroup_id, air_instance.air_id)
                    {
                        proves_out.push(proves[prover_idx]);
                    } else {
                        let base_filename_path = if *proof_type == ProofType::Compressor {
                            air_setup_folder.join("compressor").display().to_string()
                        } else {
                            air_setup_folder.join("recursive1").display().to_string()
                        };

                        // witness computation
                        let rust_lib_filename = base_filename_path.clone() + ".so";
                        let rust_lib_path = Path::new(rust_lib_filename.as_str());

                        if !rust_lib_path.exists() {
                            return Err(
                                format!("Rust lib dynamic library not found at path: {:?}", rust_lib_path).into()
                            );
                        }

                        // Load the dynamic library at runtime
                        let library = unsafe { Library::new(rust_lib_path)? };

                        // get setup
                        let setup: &proofman_common::Setup =
                            sctx.get_setup(air_instance.airgroup_id, air_instance.air_id).expect("Setup not found");

                        let p_setup: *mut c_void = setup.p_setup;
                        let p_stark_info: *mut c_void = setup.p_stark_info;

                        let n = get_stark_info_n_c(p_stark_info);
                        let offset_cm1 = get_map_offsets_c(p_stark_info, "cm1", false);

                        let total_n = get_map_totaln_c(p_stark_info);
                        let n_publics = get_stark_info_n_publics_c(p_stark_info);

                        if total_n > air_instance.buffer.len() as u64 {
                            air_instance.buffer.resize(total_n as usize, F::zero());
                            // Replace 0 with a suitable default value if needed
                        }
                        let p_address = air_instance.get_buffer_ptr() as *mut c_void;

                        let publics = vec![F::zero(); n_publics as usize];
                        let p_publics = publics.as_ptr() as *mut c_void;

                        // Load the symbol (function) from the library
                        unsafe {
                            let get_commited_pols: Symbol<GetCommitedPolsFunc> = library.get(b"getCommitedPols\0")?;

                            // Call the function
                            let dat_filename = base_filename_path.clone() + ".dat";
                            let dat_filename_str = CString::new(dat_filename.as_str()).unwrap();
                            let dat_filename_ptr = dat_filename_str.as_ptr() as *mut std::os::raw::c_char;

                            let exec_filename = base_filename_path.clone() + ".exec";
                            let exec_filename_str = CString::new(exec_filename.as_str()).unwrap();
                            let exec_filename_ptr = exec_filename_str.as_ptr() as *mut std::os::raw::c_char;

                            let mut zkin = proves[prover_idx];
                            if *proof_type == ProofType::Recursive1 {
                                let recursive2_verkey = pctx
                                    .global_info
                                    .get_air_setup_path(
                                        air_instance.airgroup_id,
                                        air_instance.air_id,
                                        &ProofType::Recursive2,
                                    )
                                    .join("recursive2")
                                    .display()
                                    .to_string()
                                    + ".verkey.json";
                                zkin = add_recursive2_verkey_c(zkin, recursive2_verkey.as_str());
                            }
                            get_commited_pols(
                                p_address,
                                p_publics,
                                zkin,
                                n,
                                n_publics,
                                offset_cm1,
                                dat_filename_ptr,
                                exec_filename_ptr,
                            );
                        }

                        let air_instance_name =
                            &pctx.global_info.airs[air_instance.airgroup_id][air_instance.air_id].name;

                        let proof_type_str = match proof_type {
                            ProofType::Compressor => "compressor",
                            ProofType::Recursive1 => "recursive1",
                            _ => panic!(),
                        };

                        let output_file_path = output_dir_path
                            .join(format!("proofs/{}_{}_{}.json", proof_type_str, air_instance_name, prover_idx));

                        let proof_file = match save_proof {
                            true => output_file_path.to_string_lossy().into_owned(),
                            false => String::from(""),
                        };

                        // prove
                        let mut p_prove =
                            gen_recursive_proof_c(p_setup, p_address, publics.as_ptr() as *mut c_void, &proof_file);

                        p_prove = publics2zkin_c(
                            p_prove,
                            p_publics,
                            global_info_file,
                            air_instance.airgroup_id as u64,
                            false,
                        );

                        proves_out.push(p_prove);
                    }
                }
            }
            ProofType::Recursive2 => {
                let sctx = SetupCtx::new(&pctx.global_info, proof_type);

                let n_airgroups = pctx.global_info.subproofs.len();
                let mut proves_recursive2: Vec<*mut c_void> = Vec::with_capacity(n_airgroups);

                for airgroup in 0..n_airgroups {
                    let air_setup_folder = pctx.global_info.get_air_setup_path(airgroup, 0, proof_type);

                    let base_filename_path = air_setup_folder.join("recursive2").display().to_string();

                    let instances = pctx.air_instance_repo.find_airgroup_instances(airgroup);
                    if instances.is_empty() {
                        let zkin_file = base_filename_path.clone() + ".null_zkin.json";
                        let null_zkin = get_zkin_ptr_c(&zkin_file);
                        proves_recursive2.push(null_zkin);
                    } else if instances.len() == 1 {
                        proves_recursive2.insert(airgroup, proves[instances[0]]);
                    } else {
                        let mut proves_recursive2_airgroup: Vec<*mut c_void> = Vec::new();

                        for instance in instances.iter() {
                            proves_recursive2_airgroup.push(proves[*instance]);
                        }

                        //create a vector of sice indices length
                        let mut alive = instances.len();
                        while alive > 1 {
                            for i in 0..alive / 2 {
                                let j = i * 2;
                                if j + 1 < alive {
                                    //initialize zkin with a void pionter
                                    let zkin: *mut std::ffi::c_void = std::ptr::null_mut();
                                    // TODO
                                    proves_recursive2_airgroup[j] = zkin;
                                }
                            }
                            alive = (alive + 1) / 2;
                            //compact elements
                            for i in 0..alive {
                                proves_recursive2_airgroup[i] = proves_recursive2_airgroup[i * 2];
                            }
                        }
                        proves_recursive2.push(proves_recursive2_airgroup[0]);
                    }
                }

                let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
                let challenges_guard = pctx.challenges.challenges.read().unwrap();

                let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
                let challenges = (*challenges_guard).as_ptr() as *mut c_void;

                let mut stark_infos_recursive2 = Vec::new();
                for (idx, _) in pctx.global_info.subproofs.iter().enumerate() {
                    stark_infos_recursive2.push(sctx.get_setup(idx, 0).unwrap().p_stark_info);
                }

                let proves_recursive2_ptr = proves_recursive2.as_mut_ptr();

                let stark_infos_recursive2_ptr = stark_infos_recursive2.as_mut_ptr();

                let zkin_final = join_zkin_final_c(
                    public_inputs,
                    challenges,
                    global_info_file,
                    proves_recursive2_ptr,
                    stark_infos_recursive2_ptr,
                );

                proves_out.push(zkin_final);
            }
            ProofType::Final => {
                let sctx = SetupCtx::new(&pctx.global_info, proof_type);

                let final_setup_folder = pctx.global_info.get_final_setup_path();
                trace!("{}   : ··· Setup Final folder: {:?}", Self::MY_NAME, final_setup_folder);

                // Check path exists and is a folder
                if !final_setup_folder.exists() {
                    panic!("Setup AIR folder not found at path: {:?}", final_setup_folder);
                }
                if !final_setup_folder.is_dir() {
                    panic!("Setup AIR path is not a folder: {:?}", final_setup_folder);
                }

                let base_filename_path = final_setup_folder.join("final").display().to_string();

                // witness computation
                let rust_lib_filename = base_filename_path.clone() + ".so";
                let rust_lib_path = Path::new(rust_lib_filename.as_str());

                if !rust_lib_path.exists() {
                    return Err(format!("Rust lib dynamic library not found at path: {:?}", rust_lib_path).into());
                }

                // Load the dynamic library at runtime
                let library = unsafe { Library::new(rust_lib_path)? };

                // get setup
                let setup: &proofman_common::Setup = sctx.get_setup(0, 0).expect("Setup not found");

                let p_setup: *mut c_void = setup.p_setup;
                let p_stark_info: *mut c_void = setup.p_stark_info;

                let n = get_stark_info_n_c(p_stark_info);
                let offset_cm1 = get_map_offsets_c(p_stark_info, "cm1", false);

                let total_n = get_map_totaln_c(p_stark_info);
                let n_publics = get_stark_info_n_publics_c(p_stark_info);

                let mut buffer = vec![F::zero(); total_n as usize];

                let p_address = buffer.as_mut_ptr() as *mut u8 as *mut c_void;

                let publics = vec![F::zero(); n_publics as usize];
                let p_publics = publics.as_ptr() as *mut c_void;

                // Load the symbol (function) from the library
                unsafe {
                    let get_commited_pols: Symbol<GetCommitedPolsFunc> = library.get(b"getCommitedPols\0")?;

                    // Call the function
                    let dat_filename = base_filename_path.clone() + ".dat";
                    let dat_filename_str = CString::new(dat_filename.as_str()).unwrap();
                    let dat_filename_ptr = dat_filename_str.as_ptr() as *mut std::os::raw::c_char;

                    let exec_filename = base_filename_path.clone() + ".exec";
                    let exec_filename_str = CString::new(exec_filename.as_str()).unwrap();
                    let exec_filename_ptr = exec_filename_str.as_ptr() as *mut std::os::raw::c_char;

                    get_commited_pols(
                        p_address,
                        p_publics,
                        proves[0],
                        n,
                        n_publics,
                        offset_cm1,
                        dat_filename_ptr,
                        exec_filename_ptr,
                    );
                }

                let proof_output_path = output_dir_path.join("proofs/final_proof.json");

                // prove
                let _p_prove = gen_recursive_proof_c(
                    p_setup,
                    p_address,
                    publics.as_ptr() as *mut c_void,
                    proof_output_path.to_string_lossy().as_ref(),
                );
            }
            ProofType::Basic => {
                panic!("Recursion proof whould not be calles for ProofType::Basic ");
            }
        }
        Ok(proves_out)
    }

    fn print_summary(pctx: Arc<ProofCtx<F>>) {
        let air_instances_repo = pctx.air_instance_repo.air_instances.read().unwrap();
        let air_instances_repo = &*air_instances_repo;

        let mut air_instances = HashMap::new();
        for air_instance in air_instances_repo.iter() {
            let air_name = pctx.global_info.airs[air_instance.airgroup_id][air_instance.air_id].clone().name;
            let air_group_name = pctx.global_info.subproofs[air_instance.airgroup_id].clone();
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
