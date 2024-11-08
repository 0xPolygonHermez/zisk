use libloading::{Library, Symbol};
use log::{info, trace};
use p3_field::Field;
use stark::{StarkBufferAllocator, StarkProver};
use proofman_starks_lib_c::{save_challenges_c, save_proof_values_c, save_publics_c};
use core::panic;
use std::fs;
use std::error::Error;
use std::mem::MaybeUninit;

use colored::*;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use transcript::FFITranscript;

use crate::{WitnessLibInitFn, WitnessLibrary};
use crate::verify_constraints_proof;
use crate::generate_recursion_proof;

use proofman_common::{ExecutionCtx, ProofCtx, ProofOptions, ProofType, Prover, SetupCtx, SetupsVadcop};

use std::os::raw::c_void;

use proofman_util::{timer_start_debug, timer_start_info, timer_stop_and_log_debug, timer_stop_and_log_info};

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
        timer_start_info!(GENERATING_VADCOP_PROOF);

        timer_start_info!(GENERATING_PROOF);

        Self::check_paths(
            &witness_lib_path,
            &rom_path,
            &public_inputs_path,
            &proving_key_path,
            &output_dir_path,
            options.verify_constraints,
        )?;
        let buffer_allocator: Arc<StarkBufferAllocator> = Arc::new(StarkBufferAllocator::new(proving_key_path.clone()));
        let ectx = ExecutionCtx::builder()
            .with_rom_path(rom_path.clone())
            .with_buffer_allocator(buffer_allocator)
            .with_verbose_mode(options.verbose_mode)
            .build();
        let ectx = Arc::new(ectx);
        // Load the witness computation dynamic library
        let library = unsafe { Library::new(&witness_lib_path)? };

        let witness_lib: Symbol<WitnessLibInitFn<F>> = unsafe { library.get(b"init_library")? };

        let mut witness_lib = witness_lib(rom_path, public_inputs_path, ectx.verbose_mode)?;

        let pctx = Arc::new(ProofCtx::create_ctx(witness_lib.pilout(), proving_key_path.clone()));

        let setups = Arc::new(SetupsVadcop::new(&pctx.global_info, options.aggregation));
        let sctx: Arc<SetupCtx<F>> = setups.sctx.clone();

        Self::initialize_witness(&mut witness_lib, pctx.clone(), ectx.clone(), sctx.clone());
        witness_lib.calculate_witness(1, pctx.clone(), ectx.clone(), sctx.clone());

        let mut dctx = ectx.dctx.write().unwrap();
        dctx.close(pctx.global_info.air_groups.len());
        let mpi_rank = dctx.rank;
        drop(dctx);
        if mpi_rank == 0 {
            Self::print_summary(pctx.clone());
        }

        Self::initialize_fixed_pols(setups.clone(), pctx.clone(), ectx.clone(), options.aggregation);

        let mut provers: Vec<Box<dyn Prover<F>>> = Vec::new();
        Self::initialize_provers(sctx.clone(), &mut provers, pctx.clone(), ectx.clone());

        if provers.is_empty() {
            return Err("No instances found".into());
        }

        let mut transcript: FFITranscript = provers[0].new_transcript();

        Self::check_stage(0, &mut provers, pctx.clone());
        for prover in provers.iter_mut() {
            prover.commit_stage(0, pctx.clone());
        }

        // Commit stages
        let num_commit_stages = pctx.global_info.n_challenges.len() as u32;
        for stage in 1..=num_commit_stages {
            Self::get_challenges(stage, &mut provers, pctx.clone(), &transcript);
            if stage != 1 {
                timer_start_debug!(CALCULATING_WITNESS);
                info!("{}: Calculating witness stage {}", Self::MY_NAME, stage);
                witness_lib.calculate_witness(stage, pctx.clone(), ectx.clone(), sctx.clone());
                timer_stop_and_log_debug!(CALCULATING_WITNESS);
            }

            Self::calculate_stage(stage, &mut provers, sctx.clone(), pctx.clone());
            Self::check_stage(stage, &mut provers, pctx.clone());

            if !options.verify_constraints {
                Self::commit_stage(stage, &mut provers, pctx.clone());
            }

            let publics_set = pctx.public_inputs.inputs_set.read().unwrap();
            for i in 0..pctx.global_info.n_publics {
                let public = pctx.global_info.publics_map.as_ref().expect("REASON").get(i).unwrap();
                if !publics_set[i] {
                    panic!("Not all publics are set: Public {} is not calculated", public.name);
                }
            }

            if !options.verify_constraints || stage < num_commit_stages {
                Self::calculate_challenges(
                    stage,
                    &mut provers,
                    pctx.clone(),
                    ectx.clone(),
                    &mut transcript,
                    options.verify_constraints,
                );
            }
        }

        witness_lib.end_proof();

        for i in 0..pctx.global_info.n_proof_values {
            if !pctx.proof_values.values_set.read().unwrap().contains_key(&i) {
                panic!(
                    "Proof cannot be generated: Proof value {} is not set",
                    pctx.global_info.proof_values_map.as_ref().expect("REASON").get(i).unwrap().name
                );
            }
        }

        if options.verify_constraints {
            return verify_constraints_proof(pctx.clone(), ectx.clone(), sctx.clone(), provers, witness_lib);
        }

        // Compute Quotient polynomial
        Self::get_challenges(num_commit_stages + 1, &mut provers, pctx.clone(), &transcript);
        Self::calculate_stage(num_commit_stages + 1, &mut provers, sctx.clone(), pctx.clone());
        Self::commit_stage(num_commit_stages + 1, &mut provers, pctx.clone());
        Self::calculate_challenges(
            num_commit_stages + 1,
            &mut provers,
            pctx.clone(),
            ectx.clone(),
            &mut transcript,
            false,
        );

        // Compute openings
        Self::opening_stages(&mut provers, pctx.clone(), sctx.clone(), ectx.clone(), &mut transcript);

        //Generate proves_out
        let proves_out = Self::finalize_proof(&mut provers, pctx.clone(), output_dir_path.to_string_lossy().as_ref());

        timer_stop_and_log_info!(GENERATING_PROOF);

        if !options.aggregation {
            return Ok(());
        }

        log::info!("{}: ··· Generating aggregated proofs", Self::MY_NAME);

        timer_start_info!(GENERATING_AGGREGATION_PROOFS);
        timer_start_info!(GENERATING_COMPRESSOR_PROOFS);
        let comp_proofs = generate_recursion_proof(
            &pctx,
            &ectx,
            setups.sctx_compressor.as_ref().unwrap().clone(),
            &proves_out,
            &ProofType::Compressor,
            output_dir_path.clone(),
            false,
        )?;
        timer_stop_and_log_info!(GENERATING_COMPRESSOR_PROOFS);
        log::info!("{}: Compressor proofs generated successfully", Self::MY_NAME);

        timer_start_info!(GENERATING_RECURSIVE1_PROOFS);
        let recursive1_proofs = generate_recursion_proof(
            &pctx,
            &ectx,
            setups.sctx_recursive1.as_ref().unwrap().clone(),
            &comp_proofs,
            &ProofType::Recursive1,
            output_dir_path.clone(),
            false,
        )?;
        timer_stop_and_log_info!(GENERATING_RECURSIVE1_PROOFS);
        log::info!("{}: Recursive1 proofs generated successfully", Self::MY_NAME);

        ectx.dctx.read().unwrap().barrier();
        timer_start_info!(GENERATING_RECURSIVE2_PROOFS);
        let sctx_recursive2 = setups.sctx_recursive2.clone();
        let recursive2_proofs = generate_recursion_proof(
            &pctx,
            &ectx,
            sctx_recursive2.as_ref().unwrap().clone(),
            &recursive1_proofs,
            &ProofType::Recursive2,
            output_dir_path.clone(),
            false,
        )?;
        timer_stop_and_log_info!(GENERATING_RECURSIVE2_PROOFS);
        log::info!("{}: Recursive2 proofs generated successfully", Self::MY_NAME);

        ectx.dctx.read().unwrap().barrier();
        if mpi_rank == 0 {
            timer_start_info!(GENERATING_FINAL_PROOFS);
            let _final_proof = generate_recursion_proof(
                &pctx,
                &ectx,
                setups.sctx_final.as_ref().unwrap().clone(),
                &recursive2_proofs,
                &ProofType::Final,
                output_dir_path.clone(),
                true,
            )?;
            timer_stop_and_log_info!(GENERATING_FINAL_PROOFS);
            log::info!("{}: Final proof generated successfully", Self::MY_NAME);
        }
        timer_stop_and_log_info!(GENERATING_AGGREGATION_PROOFS);
        timer_stop_and_log_info!(GENERATING_VADCOP_PROOF);
        log::info!("{}: Proofs generated successfully", Self::MY_NAME);
        ectx.dctx.read().unwrap().barrier();
        Ok(())
    }

    fn initialize_witness(
        witness_lib: &mut Box<dyn WitnessLibrary<F>>,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
    ) {
        timer_start_debug!(INITIALIZE_WITNESS);
        witness_lib.start_proof(pctx.clone(), ectx.clone(), sctx.clone());

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

                let airgroup_name = &pctx.global_info.air_groups[*airgroup_id];
                trace!("{}:     + AirGroup [{}] {}", Self::MY_NAME, airgroup_id, airgroup_name);

                for &air_id in &sorted_air_ids {
                    if let Some(&count) = air_map.get(air_id) {
                        let air_name = &pctx.global_info.airs[*airgroup_id][*air_id].name;
                        trace!("{}:       · {} x Air[{}] {}", Self::MY_NAME, count, air_id, air_name);
                    }
                }
            }
        }
        timer_stop_and_log_debug!(INITIALIZE_WITNESS);
    }

    fn initialize_provers(
        sctx: Arc<SetupCtx<F>>,
        provers: &mut Vec<Box<dyn Prover<F>>>,
        pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx<F>>,
    ) {
        timer_start_debug!(INITIALIZE_PROVERS);
        info!("{}: Initializing provers", Self::MY_NAME);
        for air_instance in pctx.air_instance_repo.air_instances.read().unwrap().iter() {
            let air_name = &pctx.global_info.airs[air_instance.airgroup_id][air_instance.air_id].name;
            log::debug!("{}: Initializing prover for air instance {}", Self::MY_NAME, air_name);
            let prover = Box::new(StarkProver::new(
                sctx.clone(),
                pctx.clone(),
                air_instance.airgroup_id,
                air_instance.air_id,
                air_instance.air_instance_id.unwrap(),
                air_instance.idx.unwrap(),
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

        let buff_helper: Vec<MaybeUninit<F>> = Vec::with_capacity(buff_helper_size);

        *pctx.buff_helper.buff_helper.write().unwrap() = buff_helper;
        timer_stop_and_log_debug!(INITIALIZE_PROVERS);
    }

    fn initialize_fixed_pols(
        setups: Arc<SetupsVadcop<F>>,
        pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx<F>>,
        aggregation: bool,
    ) {
        info!("{}: Initializing setup fixed pols", Self::MY_NAME);
        timer_start_debug!(INITIALIZE_SETUP);
        timer_start_debug!(INITIALIZE_CONST_POLS);

        let mut const_pols_calculated: HashMap<(usize, usize), bool> = HashMap::new();

        for air_instance in pctx.air_instance_repo.air_instances.read().unwrap().iter() {
            let (airgroup_id, air_id) = (air_instance.airgroup_id, air_instance.air_id);
            const_pols_calculated.entry((airgroup_id, air_id)).or_insert_with(|| {
                let setup = setups.sctx.get_setup(airgroup_id, air_id);
                setup.load_const_pols(&pctx.global_info, &ProofType::Basic);
                setup.load_const_pols_tree(&pctx.global_info, &ProofType::Basic, false);
                true
            });
        }

        timer_stop_and_log_debug!(INITIALIZE_CONST_POLS);

        if aggregation {
            info!("{}: Initializing setup fixed pols aggregation", Self::MY_NAME);

            let sctx_compressor = setups.sctx_compressor.as_ref().unwrap().clone();
            let sctx_recursive1 = setups.sctx_recursive1.as_ref().unwrap().clone();
            let sctx_recursive2 = setups.sctx_recursive2.as_ref().unwrap().clone();
            let sctx_final = setups.sctx_final.as_ref().unwrap().clone();

            timer_start_debug!(INITIALIZE_CONST_POLS_COMPRESSOR);
            info!("{}: ··· Initializing setup fixed pols compressor", Self::MY_NAME);
            let mut const_pols_calculated_compressor: HashMap<(usize, usize), bool> = HashMap::new();

            for air_instance in pctx.air_instance_repo.air_instances.read().unwrap().iter() {
                let (airgroup_id, air_id) = (air_instance.airgroup_id, air_instance.air_id);
                if pctx.global_info.get_air_has_compressor(airgroup_id, air_id)
                    && !const_pols_calculated_compressor.contains_key(&(airgroup_id, air_id))
                {
                    let setup = sctx_compressor.get_setup(airgroup_id, air_id);
                    setup.load_const_pols(&pctx.global_info, &ProofType::Compressor);
                    setup.load_const_pols_tree(&pctx.global_info, &ProofType::Compressor, false);
                    const_pols_calculated_compressor.insert((airgroup_id, air_id), true);
                }
            }
            timer_stop_and_log_debug!(INITIALIZE_CONST_POLS_COMPRESSOR);

            timer_start_debug!(INITIALIZE_CONST_POLS_RECURSIVE1);
            info!("{}: ··· Initializing setup fixed pols recursive1", Self::MY_NAME);
            let mut const_pols_calculated_recursive1: HashMap<(usize, usize), bool> = HashMap::new();
            for air_instance in pctx.air_instance_repo.air_instances.read().unwrap().iter() {
                let (airgroup_id, air_id) = (air_instance.airgroup_id, air_instance.air_id);
                const_pols_calculated_recursive1.entry((airgroup_id, air_id)).or_insert_with(|| {
                    let setup = sctx_recursive1.get_setup(airgroup_id, air_id);
                    setup.load_const_pols(&pctx.global_info, &ProofType::Recursive1);
                    setup.load_const_pols_tree(&pctx.global_info, &ProofType::Recursive1, false);
                    true
                });
            }
            timer_stop_and_log_debug!(INITIALIZE_CONST_POLS_RECURSIVE1);

            timer_start_debug!(INITIALIZE_CONST_POLS_RECURSIVE2);
            info!("{}: ··· Initializing setup fixed pols recursive2", Self::MY_NAME);
            let n_airgroups = pctx.global_info.air_groups.len();
            for airgroup in 0..n_airgroups {
                let setup = sctx_recursive2.get_setup(airgroup, 0);
                setup.load_const_pols(&pctx.global_info, &ProofType::Recursive2);
                setup.load_const_pols_tree(&pctx.global_info, &ProofType::Recursive2, false);
            }
            timer_stop_and_log_debug!(INITIALIZE_CONST_POLS_RECURSIVE2);

            timer_start_debug!(INITIALIZE_CONST_POLS_FINAL);
            info!("{}: ··· Initializing setup fixed pols final", Self::MY_NAME);
            let setup = sctx_final.get_setup(0, 0);
            setup.load_const_pols(&pctx.global_info, &ProofType::Final);
            setup.load_const_pols_tree(&pctx.global_info, &ProofType::Final, false);
            timer_stop_and_log_debug!(INITIALIZE_CONST_POLS_FINAL);
        }
        timer_stop_and_log_debug!(INITIALIZE_SETUP);
    }

    pub fn calculate_stage(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        setup_ctx: Arc<SetupCtx<F>>,
        proof_ctx: Arc<ProofCtx<F>>,
    ) {
        if stage as usize == proof_ctx.global_info.n_challenges.len() + 1 {
            info!("{}: Calculating Quotient Polynomials", Self::MY_NAME);
            timer_start_debug!(CALCULATING_QUOTIENT_POLYNOMIAL);
            for prover in provers.iter_mut() {
                prover.calculate_stage(stage, setup_ctx.clone(), proof_ctx.clone());
            }
            timer_stop_and_log_debug!(CALCULATING_QUOTIENT_POLYNOMIAL);
        } else {
            info!("{}: Calculating stage {}", Self::MY_NAME, stage);
            timer_start_debug!(CALCULATING_STAGE);
            for prover in provers.iter_mut() {
                prover.calculate_stage(stage, setup_ctx.clone(), proof_ctx.clone());
            }
            timer_stop_and_log_debug!(CALCULATING_STAGE);
        }
    }

    pub fn check_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>], proof_ctx: Arc<ProofCtx<F>>) {
        log::debug!("{}: Checking stage can be calculated", Self::MY_NAME);
        for prover in provers.iter_mut() {
            prover.check_stage(stage, proof_ctx.clone());
        }
    }

    pub fn commit_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>], proof_ctx: Arc<ProofCtx<F>>) {
        if stage as usize == proof_ctx.global_info.n_challenges.len() + 1 {
            info!("{}: Committing stage Q", Self::MY_NAME);
        } else {
            info!("{}: Committing stage {}", Self::MY_NAME, stage);
        }

        timer_start_debug!(COMMITING_STAGE);
        for prover in provers.iter_mut() {
            prover.commit_stage(stage, proof_ctx.clone());
        }
        timer_stop_and_log_debug!(COMMITING_STAGE);
    }

    fn hash_b_tree(prover: &dyn Prover<F>, values: Vec<Vec<F>>) -> Vec<F> {
        if values.len() == 1 {
            return values[0].clone();
        }

        let mut result = Vec::new();

        for i in (0..values.len() - 1).step_by(2) {
            let mut buffer = values[i].clone();
            buffer.extend(values[i + 1].clone());

            let is_value1_zero = values[i].iter().all(|x| *x == F::zero());
            let is_value2_zero = values[i + 1].iter().all(|x| *x == F::zero());

            let value;
            if is_value1_zero && is_value2_zero {
                value = vec![F::zero(); 4];
            } else if is_value1_zero {
                value = values[i + 1].clone();
            } else if is_value2_zero {
                value = values[i].clone();
            } else {
                value = prover.calculate_hash(buffer);
            }

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
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx<F>>,
        transcript: &mut FFITranscript,
        verify_constraints: bool,
    ) {
        if stage == 1 {
            let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
            let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;

            transcript.add_elements(public_inputs, pctx.global_info.n_publics);
        }

        let dctx = ectx.dctx.read().unwrap();

        // calculate my roots
        let mut roots: Vec<u64> = vec![0; 4 * provers.len()];
        for (i, prover) in provers.iter_mut().enumerate() {
            // Important we need the roots in u64 in order to distribute them
            let values = prover.get_transcript_values_u64(stage as u64, pctx.clone());
            if values.is_empty() {
                panic!("No transcript values found for prover {}", i);
            }
            roots[i * 4..(i + 1) * 4].copy_from_slice(&values)
        }
        // get all roots
        let all_roots = dctx.distribute_roots(roots);

        // add challenges to transcript in order
        for group_idxs in dctx.my_groups.iter() {
            if verify_constraints {
                let dummy_elements = [F::zero(), F::one(), F::two(), F::neg_one()];
                transcript.add_elements(dummy_elements.as_ptr() as *mut c_void, 4);
            } else {
                let mut values = Vec::new();
                for idx in group_idxs.iter() {
                    let value = vec![
                        F::from_wrapped_u64(all_roots[*idx]),
                        F::from_wrapped_u64(all_roots[*idx + 1]),
                        F::from_wrapped_u64(all_roots[*idx + 2]),
                        F::from_wrapped_u64(all_roots[*idx + 3]),
                    ];
                    values.push(value);
                }
                if !values.is_empty() {
                    let value = Self::hash_b_tree(&*provers[0], values);
                    transcript.add_elements(value.as_ptr() as *mut c_void, value.len());
                }
            }
        }
        drop(dctx);
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
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        ectx: Arc<ExecutionCtx<F>>,
        transcript: &mut FFITranscript,
    ) {
        let num_commit_stages = pctx.global_info.n_challenges.len() as u32;
        let dctx = ectx.dctx.read().unwrap();

        // Calculate evals
        timer_start_debug!(CALCULATING_EVALS);
        Self::get_challenges(pctx.global_info.n_challenges.len() as u32 + 2, provers, pctx.clone(), transcript);
        for group_idx in dctx.my_air_groups.iter() {
            provers[group_idx[0]].calculate_lev(pctx.clone());
            for idx in group_idx.iter() {
                provers[*idx].opening_stage(1, sctx.clone(), pctx.clone());
            }
        }
        timer_stop_and_log_debug!(CALCULATING_EVALS);
        Self::calculate_challenges(num_commit_stages + 2, provers, pctx.clone(), ectx.clone(), transcript, false);

        // Calculate fri polynomial
        Self::get_challenges(pctx.global_info.n_challenges.len() as u32 + 3, provers, pctx.clone(), transcript);
        info!("{}: Calculating FRI Polynomials", Self::MY_NAME);
        timer_start_debug!(CALCULATING_FRI_POLINOMIAL);
        for group_idx in dctx.my_air_groups.iter() {
            provers[group_idx[0]].calculate_xdivxsub(pctx.clone());
            for idx in group_idx.iter() {
                provers[*idx].opening_stage(2, sctx.clone(), pctx.clone());
            }
        }
        timer_stop_and_log_debug!(CALCULATING_FRI_POLINOMIAL);
        drop(dctx);

        let global_steps_fri: Vec<usize> = pctx.global_info.steps_fri.iter().map(|step| step.n_bits).collect();
        let num_opening_stages = global_steps_fri.len() as u32;

        timer_start_debug!(CALCULATING_FRI);
        for opening_id in 0..=num_opening_stages {
            timer_start_debug!(CALCULATING_FRI_STEP);
            Self::get_challenges(
                pctx.global_info.n_challenges.len() as u32 + 4 + opening_id,
                provers,
                pctx.clone(),
                transcript,
            );
            if opening_id == num_opening_stages - 1 {
                info!(
                    "{}: Calculating final FRI polynomial at {}",
                    Self::MY_NAME,
                    global_steps_fri[opening_id as usize]
                );
            } else if opening_id == num_opening_stages {
                info!("{}: Calculating FRI queries", Self::MY_NAME);
            } else {
                info!(
                    "{}: Calculating FRI folding from {} to {}",
                    Self::MY_NAME,
                    global_steps_fri[opening_id as usize],
                    global_steps_fri[opening_id as usize + 1]
                );
            }
            for prover in provers.iter_mut() {
                prover.opening_stage(opening_id + 3, sctx.clone(), pctx.clone());
            }
            if opening_id < num_opening_stages {
                Self::calculate_challenges(
                    pctx.global_info.n_challenges.len() as u32 + 4 + opening_id,
                    provers,
                    pctx.clone(),
                    ectx.clone(),
                    transcript,
                    false,
                );
            }
            timer_stop_and_log_debug!(CALCULATING_FRI_STEP);
        }
        timer_stop_and_log_debug!(CALCULATING_FRI);
    }

    fn finalize_proof(
        provers: &mut [Box<dyn Prover<F>>],
        proof_ctx: Arc<ProofCtx<F>>,
        output_dir: &str,
    ) -> Vec<*mut c_void> {
        timer_start_debug!(FINALIZING_PROOF);
        let mut proves = Vec::new();
        for prover in provers.iter_mut() {
            proves.push(prover.get_zkin_proof(proof_ctx.clone(), output_dir));
            prover.free();
        }
        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();
        let proof_values_guard = proof_ctx.proof_values.values.read().unwrap();

        let n_publics = proof_ctx.global_info.n_publics as u64;
        let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
        let challenges = (*challenges_guard).as_ptr() as *mut c_void;

        let n_proof_values = proof_ctx.global_info.n_proof_values as u64;
        let proof_values = (*proof_values_guard).as_ptr() as *mut c_void;

        let global_info_path = proof_ctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
        let global_info_file: &str = global_info_path.to_str().unwrap();

        save_publics_c(n_publics, public_inputs, output_dir);

        save_proof_values_c(n_proof_values, proof_values, output_dir);

        save_challenges_c(challenges, global_info_file, output_dir);

        timer_stop_and_log_debug!(FINALIZING_PROOF);
        proves
    }

    fn print_summary(pctx: Arc<ProofCtx<F>>) {
        let air_instances_repo = pctx.air_instance_repo.air_instances.read().unwrap();
        let air_instances_repo = &*air_instances_repo;

        let mut air_instances = HashMap::new();
        for air_instance in air_instances_repo.iter() {
            let air_name = pctx.global_info.airs[air_instance.airgroup_id][air_instance.air_id].clone().name;
            let air_group_name = pctx.global_info.air_groups[air_instance.airgroup_id].clone();
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
                info!("{}:       {}", Self::MY_NAME, format!("· {} x Air [{}]", count, air_name).bright_white().bold());
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
        verify_constraints: bool,
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

        if !verify_constraints && !output_dir_path.exists() {
            fs::create_dir_all(output_dir_path)
                .map_err(|err| format!("Failed to create output directory: {:?}", err))?;
        }

        Ok(())
    }
}
