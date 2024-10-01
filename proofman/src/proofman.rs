use libloading::{Library, Symbol};
use log::{info, trace};
use p3_field::Field;
use stark::{StarkBufferAllocator, StarkProver};
use proofman_starks_lib_c::{save_challenges_c, save_publics_c};
use std::fs;
use proofman_starks_lib_c::*;
use mpi::traits::Communicator;
use mpi::collective::CommunicatorCollectives;
use std::error::Error;

use std::time::Instant;
use std::process;
use std::sync::Barrier;

use colored::*;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use transcript::FFITranscript;

use crate::{WitnessLibrary, WitnessLibInitFn};
use crate::verify_constraints_proof;
use crate::generate_recursion_proof;

use proofman_common::{ExecutionCtx, ProofCtx, ProofOptions, ProofType, Prover, SetupCtx};

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
        options: ProofOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        set_log_level_c(options.verbose_mode.into());

        let (universe, _threading) = mpi::initialize_with_threading(mpi::Threading::Multiple).unwrap();
        let world: mpi::topology::SimpleCommunicator = universe.world();
        let rank = world.rank();
        let size = world.size();
        let master_rank = 0;
        world.barrier();
        let time_setup1 = Instant::now();
        Self::check_paths(
            &witness_lib_path,
            &rom_path,
            &public_inputs_path,
            &proving_key_path,
            &output_dir_path,
            options.debug_mode,
        )?;

        let buffer_allocator: Arc<StarkBufferAllocator> = Arc::new(StarkBufferAllocator::new(proving_key_path.clone()));
        let mut ectx = ExecutionCtx::builder()
            .with_rom_path(rom_path)
            .with_buffer_allocator(buffer_allocator)
            .with_verbose_mode(options.verbose_mode)
            .build();
        ectx.rank = rank;
        ectx.n_processes = size;
        let ectx = Arc::new(ectx);

        // Load the witness computation dynamic library
        let library = unsafe { Library::new(&witness_lib_path)? };

        let witness_lib: Symbol<WitnessLibInitFn<F>> = unsafe { library.get(b"init_library")? };

        let mut witness_lib = witness_lib(ectx.clone(), public_inputs_path)?;

        let pctx = Arc::new(ProofCtx::create_ctx(witness_lib.pilout(), proving_key_path.clone()));

        let sctx: Arc<SetupCtx> = Arc::new(SetupCtx::new(&pctx.global_info, &ProofType::Basic));

        world.barrier();
        let elapsed_setup1 = time_setup1.elapsed();
        world.barrier();
        let time_wit = Instant::now();
        Self::initialize_witness(&mut witness_lib, pctx.clone(), ectx.clone(), sctx.clone());
        witness_lib.calculate_witness(1, pctx.clone(), ectx.clone(), sctx.clone());
        world.barrier();
        let elapsed_wit = time_wit.elapsed();
        println!("Elapsed WC:    {:?}", elapsed_wit);
        world.barrier();
        process::exit(0);
        
        let time_prove: Instant = Instant::now();

        if master_rank == rank {
            Self::print_summary(pctx.clone());
        }

        let mut provers: Vec<Box<dyn Prover<F>>> = Vec::new();
        let n_provers = Self::initialize_provers(sctx.clone(), &mut provers, pctx.clone(), rank, size);

        if provers.is_empty() {
            panic!("No provers found for rank {}", rank);
            //TODO: the process should be retired from the comunicator...
        }

        let mut transcript: FFITranscript = provers[0].new_transcript();
        Self::calculate_challenges(0, &mut provers, pctx.clone(), &mut transcript, 0, n_provers, &world);

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
                Self::calculate_challenges(
                    stage,
                    &mut provers,
                    pctx.clone(),
                    &mut transcript,
                    options.debug_mode,
                    n_provers,
                    &world,
                );
            }
        }

        witness_lib.end_proof();

        if options.debug_mode != 0 {
            verify_constraints_proof(pctx, ectx, sctx, provers, witness_lib, options);
            return Ok(());
        }

        // Compute Quotient polynomial
        Self::get_challenges(num_commit_stages + 1, &mut provers, pctx.clone(), &transcript);
        Self::calculate_stage(num_commit_stages + 1, &mut provers, pctx.clone());
        Self::commit_stage(num_commit_stages + 1, &mut provers, pctx.clone());
        Self::calculate_challenges(
            num_commit_stages + 1,
            &mut provers,
            pctx.clone(),
            &mut transcript,
            0,
            n_provers,
            &world,
        );

        // Compute openings
        Self::opening_stages(&mut provers, pctx.clone(), sctx.clone(), &mut transcript, n_provers, &world);

        world.barrier();
        let elapsed_prove = time_prove.elapsed();
        if rank == master_rank {
            println!("Elapsed setup: {:?}", elapsed_setup1);
            println!("Elapsed WC:    {:?}", elapsed_wit);
            println!("Elapsed prove: {:?}", elapsed_prove);
            println!("Elapsed total: {:?}", elapsed_setup1 + elapsed_wit + elapsed_prove);
        }
        //Generate proves_out
        let proves_out = Self::finalize_proof(
            &mut provers,
            pctx.clone(),
            output_dir_path.to_string_lossy().as_ref(),
            options.aggregation,
            options.save_proofs,
        );
        world.barrier();
        if !options.aggregation {
            return Ok(());
        }

        let comp_proofs = generate_recursion_proof(
            &pctx,
            &proves_out,
            &ProofType::Compressor,
            output_dir_path.clone(),
            options.save_proofs,
        )?;
        println!("Compressor proofs generated successfully");

        let recursive1_proofs = generate_recursion_proof(
            &pctx,
            &comp_proofs,
            &ProofType::Recursive1,
            output_dir_path.clone(),
            options.save_proofs,
        )?;
        println!("Recursive1 proofs generated successfully");

        let recursive2_proofs = generate_recursion_proof(
            &pctx,
            &recursive1_proofs,
            &ProofType::Recursive2,
            output_dir_path.clone(),
            options.save_proofs,
        )?;
        println!("Recursive2 proofs generated successfully");

        let _final_proof =
            generate_recursion_proof(&pctx, &recursive2_proofs, &ProofType::Final, output_dir_path.clone(), true)?;
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

    fn initialize_provers(
        sctx: Arc<SetupCtx>,
        provers: &mut Vec<Box<dyn Prover<F>>>,
        pctx: Arc<ProofCtx<F>>,
        rank: i32,
        size: i32,
    ) -> usize {
        timer_start!(INITIALIZING_PROVERS);
        info!("{}: ··· INITIALIZING PROVER CLIENTS", Self::MY_NAME);
        let mut cont = 0;
        for (air_instance_idx, air_instance) in pctx.air_instance_repo.air_instances.read().unwrap().iter().enumerate()
        {
            let segment_idx = air_instance.air_segment_id.unwrap_or(0); // Only for main proof
            cont += 1;
            if segment_idx as i32 % size != rank {
                continue;
            }
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
                air_instance_idx,
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
        cont
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

    fn hash_b_tree(prover: &dyn Prover<F>, values: Vec<Vec<F>>) -> Vec<F> {
        if values.len() == 1 {
            return values[0].clone();
        }

        let mut result = Vec::new();

        for i in (0..values.len() - 1).step_by(2) {
            let mut buffer = values[i].clone();
            buffer.extend(values[i + 1].clone());
            let value = prover.calculate_hash(buffer);
            result.push(value);
        }

        if values.len() % 2 != 0 {
            result.push(values[values.len() - 1].clone());
        }

        Self::hash_b_tree(prover, result)
    }

    fn calculate_challenges(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        proof_ctx: Arc<ProofCtx<F>>,
        transcript: &mut FFITranscript,
        debug_mode: u64,
        n_provers: usize,
        world: &mpi::topology::SimpleCommunicator,
    ) {
        info!("{}: ··· CALCULATING CHALLENGES FOR STAGE {}", Self::MY_NAME, stage);

        let size = world.size();
        if size == 1 {
            let airgroups = proof_ctx.global_info.subproofs.clone();
            for (airgroup_id, _airgroup) in airgroups.iter().enumerate() {
                if debug_mode != 0 {
                    let dummy_elements = [F::zero(), F::one(), F::two(), F::neg_one()];
                    transcript.add_elements(dummy_elements.as_ptr() as *mut c_void, 4);
                } else {
                    let airgroup_instances = proof_ctx.air_instance_repo.find_airgroup_instances(airgroup_id);

                    if !airgroup_instances.is_empty() {
                        let mut values = Vec::new();
                        for prover_idx in airgroup_instances.iter() {
                            let value = provers[*prover_idx].get_transcript_values(stage as u64, proof_ctx.clone());
                            values.push(value);
                        }
                        if !values.is_empty() {
                            let value = Self::hash_b_tree(&*provers[airgroup_instances[0]], values);
                            transcript.add_elements(value.as_ptr() as *mut c_void, value.len());
                        }
                    }
                }
            }
        } else {
            // max number of roots
            let max_roots = (n_provers as i32 + size - 1) / size;

            // calculate my roots
            let mut roots: Vec<u64> = vec![0; 4 * max_roots as usize];
            for (i, prover) in provers.iter_mut().enumerate() {
                //prover.get_root(stage as u64, proof_ctx.clone(), &mut roots[i * 4..(i + 1) * 4]);
                let values = prover.get_transcript_values_u64(stage as u64, proof_ctx.clone());
                if values.is_empty() {
                    panic!("No transcript values found for prover {}", i);
                }
                roots[i * 4..(i + 1) * 4].copy_from_slice(&values)
            }

            // Use all ghater
            let mut all_roots: Vec<u64> = vec![0; 4 * max_roots as usize * size as usize];
            world.all_gather_into(&roots, &mut all_roots);

            // add challenges to transcript
            let airgroups = proof_ctx.global_info.subproofs.clone();
            for (airgroup_id, _airgroup) in airgroups.iter().enumerate() {
                if debug_mode != 0 {
                    let dummy_elements = [F::zero(), F::one(), F::two(), F::neg_one()];
                    transcript.add_elements(dummy_elements.as_ptr() as *mut c_void, 4);
                } else {
                    let airgroup_instances = proof_ctx.air_instance_repo.find_airgroup_instances(airgroup_id);
                    if !airgroup_instances.is_empty() {
                        let mut values: Vec<Vec<F>> = Vec::new();
                        for air_idx in airgroup_instances.iter() {
                            let mut value = Vec::new();
                            let air_instance = &proof_ctx.air_instance_repo.air_instances.read().unwrap()[*air_idx];
                            let segment_idx = air_instance.air_segment_id.unwrap_or(0); // Only for main proof
                            let root_rank = segment_idx % size as usize;
                            let root_idx = segment_idx / size as usize;
                            let root_ptr = &all_roots[root_rank * 4 * max_roots as usize + root_idx * 4
                                ..root_rank * 4 * max_roots as usize + root_idx * 4 + 4];

                            value.push(F::from_wrapped_u64(root_ptr[0]));
                            value.push(F::from_wrapped_u64(root_ptr[1]));
                            value.push(F::from_wrapped_u64(root_ptr[2]));
                            value.push(F::from_wrapped_u64(root_ptr[3]));

                            values.push(value);
                        }
                        if !values.is_empty() {
                            let value = Self::hash_b_tree(&*provers[0], values);
                            transcript.add_elements(value.as_ptr() as *mut c_void, value.len());
                        }
                    }
                }
            }
        }

        if stage == 0 {
            let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
            let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;

            transcript.add_elements(public_inputs, proof_ctx.global_info.n_publics);
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
        n_provers: usize,
        world: &mpi::topology::SimpleCommunicator,
    ) {
        let setup_airs = setup_ctx.get_setup_airs();
        let num_commit_stages = proof_ctx.global_info.n_challenges.len() as u32;
        let size = world.size();
        let rank = world.rank();

        // Calculate evals
        Self::get_challenges(num_commit_stages + 2, provers, proof_ctx.clone(), transcript);
        for (airgroup_id, airgroup) in setup_airs.iter().enumerate() {
            for air_id in airgroup.iter() {
                let air_instances_idx: Vec<usize> =
                    proof_ctx.air_instance_repo.find_air_instances(airgroup_id, *air_id);
                if !air_instances_idx.is_empty() {
                    let mut is_first = true;
                    for idx in air_instances_idx {
                        let segment_idx =
                            &proof_ctx.air_instance_repo.air_instances.read().unwrap()[idx].air_segment_id.unwrap();
                        if *segment_idx as i32 % size == rank {
                            let loc_idx = segment_idx / size as usize;
                            if is_first {
                                provers[loc_idx].calculate_lev(proof_ctx.clone());
                                is_first = false;
                            }
                            info!("{}: Opening stage {}, for prover {}", Self::MY_NAME, 1, idx);
                            provers[loc_idx].opening_stage(1, proof_ctx.clone());
                        }
                    }
                }
            }
        }

        Self::calculate_challenges(num_commit_stages + 2, provers, proof_ctx.clone(), transcript, 0, n_provers, world);

        // Calculate fri polynomial
        Self::get_challenges(num_commit_stages + 3, provers, proof_ctx.clone(), transcript);
        for (airgroup_id, airgroup) in setup_airs.iter().enumerate() {
            for air_id in airgroup.iter() {
                let air_instances_idx: Vec<usize> =
                    proof_ctx.air_instance_repo.find_air_instances(airgroup_id, *air_id);
                if !air_instances_idx.is_empty() {
                    let mut is_first = true;
                    for idx in air_instances_idx {
                        let segment_idx =
                            &proof_ctx.air_instance_repo.air_instances.read().unwrap()[idx].air_segment_id.unwrap();
                        if *segment_idx as i32 % size == rank {
                            let loc_idx = segment_idx / size as usize;
                            if is_first {
                                provers[loc_idx].calculate_xdivxsub(proof_ctx.clone());
                                is_first = false;
                            }
                            info!("{}: Opening stage {}, for prover {}", Self::MY_NAME, 2, idx);
                            provers[loc_idx].opening_stage(2, proof_ctx.clone());
                        }
                    }
                }
            }
        }

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
                    n_provers,
                    world,
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

        info!("{}: --- PROOF INSTANCES SUMMARY ------------------------", Self::MY_NAME);
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
        info!("{}: --- PROOF INSTANCES SUMMARY ------------------------", Self::MY_NAME);
    }

    fn check_paths(
        witness_lib_path: &PathBuf,
        rom_path: &Option<PathBuf>,
        public_inputs_path: &Option<PathBuf>,
        proving_key_path: &PathBuf,
        output_dir_path: &PathBuf,
        debug_mode: u64,
    ) -> Result<(), Box<dyn Error>> {
        // Check witness_lib path exists
        if !witness_lib_path.exists() {
            return Err(format!("Witness computation dynamic library not found at path: {:?}", witness_lib_path).into());
        }

        // Check rom_path path exists
        if let Some(rom_path) = rom_path {
            if !rom_path.exists() {
                return Err(format!("ROM file not found at path: {:?}", rom_path).into());
            }
        }

        // Check public_inputs_path is a folder
        if let Some(publics_path) = public_inputs_path {
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
            fs::create_dir_all(output_dir_path)
                .map_err(|err| format!("Failed to create output directory: {:?}", err))?;
        }

        Ok(())
    }
}