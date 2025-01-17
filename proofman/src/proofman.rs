use libloading::{Library, Symbol};
use log::info;
use p3_field::PrimeField;
use stark::StarkProver;
use proofman_starks_lib_c::{
    free_provers_c, fri_proof_get_zkinproofs_c, save_challenges_c, save_proof_values_c, save_publics_c,
};
use std::fs;
use std::error::Error;

use colored::*;

use std::{
    collections::{HashSet, HashMap},
    path::PathBuf,
    sync::Arc,
};

use transcript::FFITranscript;

use witness::{WitnessLibInitFn, WitnessManager};

use crate::{
    generate_fflonk_snark_proof, generate_recursivef_proof, generate_vadcop_final_proof,
    generate_vadcop_recursive1_proof, generate_vadcop_recursive2_proof, get_buff_sizes, verify_basic_proofs,
    verify_constraints_proof, verify_proof,
};

use proofman_common::{
    format_bytes, skip_prover_instance, ProofCtx, ProofOptions, ProofType, Prover, SetupCtx, SetupsVadcop,
};

use std::os::raw::c_void;

use proofman_util::{
    create_buffer_fast, timer_start_debug, timer_start_info, timer_stop_and_log_debug, timer_stop_and_log_info,
    timer_stop_and_log_trace, timer_start_trace,
};

pub struct ProofMan<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + 'static> ProofMan<F> {
    const MY_NAME: &'static str = "ProofMan";

    pub fn generate_proof(
        witness_lib_path: PathBuf,
        rom_path: Option<PathBuf>,
        public_inputs_path: Option<PathBuf>,
        proving_key_path: PathBuf,
        output_dir_path: PathBuf,
        options: ProofOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        timer_start_info!(INITIALIZING_PROOFMAN);
        Self::check_paths(
            &witness_lib_path,
            &rom_path,
            &public_inputs_path,
            &proving_key_path,
            &output_dir_path,
            options.verify_constraints,
        )?;

        let mut pctx: ProofCtx<F> = ProofCtx::create_ctx(proving_key_path.clone(), options);

        let setups = Arc::new(SetupsVadcop::new(&pctx.global_info, pctx.options.aggregation, pctx.options.final_snark));
        let sctx: Arc<SetupCtx> = setups.sctx.clone();

        pctx.set_weights(&sctx);

        let pctx = Arc::new(pctx);

        timer_stop_and_log_info!(INITIALIZING_PROOFMAN);

        timer_start_info!(GENERATING_WITNESS);
        let wcm = Arc::new(WitnessManager::new(pctx.clone(), sctx.clone(), rom_path, public_inputs_path));

        Self::initialize_witness(witness_lib_path, wcm.clone())?;
        wcm.calculate_witness(1);

        Self::initialize_fixed_pols(setups.clone(), pctx.clone(), true);

        let mpi_rank = pctx.dctx_get_rank();
        let n_processes = pctx.dctx_get_n_processes();

        pctx.dctx_close();

        if n_processes > 1 {
            let (average_weight, max_weight, min_weight, max_deviation) = pctx.dctx_load_balance_info();
            log::info!(
                "{}: Load balance. Average: {} max: {} min: {} deviation: {}",
                Self::MY_NAME,
                average_weight,
                max_weight,
                min_weight,
                max_deviation
            );
        }

        if mpi_rank == 0 {
            Self::print_global_summary(pctx.clone(), setups.sctx.clone());
        }

        if n_processes > 1 {
            Self::print_summary(pctx.clone(), setups.sctx.clone());
        }

        timer_stop_and_log_info!(GENERATING_WITNESS);

        timer_start_info!(GENERATING_VADCOP_PROOF);

        timer_start_info!(GENERATING_PROOF);

        let mut provers: Vec<Box<dyn Prover<F>>> = Vec::new();
        Self::initialize_provers(sctx.clone(), &mut provers, pctx.clone());

        if provers.is_empty() {
            return Err("No instances found".into());
        }

        let mut transcript: FFITranscript = provers[0].new_transcript();

        timer_start_debug!(COMMITING_STAGE_0);
        let mut custom_publics = Vec::new();
        for prover in provers.iter_mut() {
            let publics = prover.commit_custom_commits_stage(0, pctx.clone());
            custom_publics.extend(publics);
        }

        pctx.dctx_distribute_publics(custom_publics);

        timer_stop_and_log_debug!(COMMITING_STAGE_0);

        // Commit stages
        let num_commit_stages = pctx.global_info.n_challenges.len() as u32;
        for stage in 1..=num_commit_stages {
            Self::get_challenges(stage, &mut provers, pctx.clone(), &transcript);
            if stage != 1 {
                timer_start_info!(CALCULATING_WITNESS);
                info!("{}: Calculating witness stage {}", Self::MY_NAME, stage);
                wcm.calculate_witness(stage);
                timer_stop_and_log_info!(CALCULATING_WITNESS);
            }

            Self::calculate_stage(stage, &mut provers, sctx.clone(), pctx.clone());

            if !pctx.options.verify_constraints {
                timer_start_trace!(STARK_COMMIT_STAGE_, stage);
                Self::commit_stage(stage, &mut provers, pctx.clone());
                timer_stop_and_log_trace!(STARK_COMMIT_STAGE_, stage);
            }

            if !pctx.options.verify_constraints || stage < num_commit_stages {
                Self::calculate_challenges(stage, &mut provers, pctx.clone(), &mut transcript);
            }
        }

        wcm.end_proof();

        if pctx.options.verify_constraints {
            wcm.debug();
            return verify_constraints_proof(pctx.clone(), sctx.clone(), &mut provers);
        }

        let pctx_ = pctx.clone();
        std::thread::spawn(move || {
            pctx_.free_traces();
        });

        // Compute Quotient polynomial
        Self::get_challenges(num_commit_stages + 1, &mut provers, pctx.clone(), &transcript);
        Self::calculate_stage(num_commit_stages + 1, &mut provers, sctx.clone(), pctx.clone());
        Self::commit_stage(num_commit_stages + 1, &mut provers, pctx.clone());
        Self::calculate_challenges(num_commit_stages + 1, &mut provers, pctx.clone(), &mut transcript);

        // Compute openings
        Self::opening_stages(&mut provers, pctx.clone(), sctx.clone(), &mut transcript);

        timer_stop_and_log_info!(GENERATING_PROOF);

        //Generate proves_out
        let proves_out = Self::get_proofs(&mut provers, pctx.clone(), output_dir_path.to_string_lossy().as_ref());

        let mut valid_proofs = false;
        if !pctx.options.aggregation {
            valid_proofs = verify_basic_proofs(&mut provers, proves_out.clone(), pctx.clone(), sctx.clone());
        }

        Self::free_provers(&mut provers, pctx.clone());

        if !pctx.options.aggregation {
            if valid_proofs {
                return Ok(());
            } else {
                return Err("Basic proofs were not verified".into());
            }
        }

        info!("{}: ··· Generating aggregated proofs", Self::MY_NAME);

        let (circom_witness_size, publics_size, trace_size, prover_buffer_size) =
            get_buff_sizes(pctx.clone(), setups.clone())?;
        let mut circom_witness: Vec<F> = create_buffer_fast(circom_witness_size);
        let publics: Vec<F> = create_buffer_fast(publics_size);
        let trace: Vec<F> = create_buffer_fast(trace_size);
        let prover_buffer: Vec<F> = create_buffer_fast(prover_buffer_size);

        timer_start_info!(GENERATING_AGGREGATION_PROOFS);
        timer_start_info!(GENERATING_COMPRESSOR_AND_RECURSIVE1_PROOFS);
        let recursive1_proofs = generate_vadcop_recursive1_proof(
            &pctx,
            setups.clone(),
            &proves_out,
            &mut circom_witness,
            &publics,
            &trace,
            &prover_buffer,
            output_dir_path.clone(),
        )?;
        timer_stop_and_log_info!(GENERATING_COMPRESSOR_AND_RECURSIVE1_PROOFS);
        info!("{}: Compressor and recursive1 proofs generated successfully", Self::MY_NAME);

        pctx.dctx.read().unwrap().barrier();
        timer_start_info!(GENERATING_RECURSIVE2_PROOFS);
        let sctx_recursive2 = setups.sctx_recursive2.clone();
        let recursive2_proof = generate_vadcop_recursive2_proof(
            &pctx,
            sctx_recursive2.as_ref().unwrap().clone(),
            &recursive1_proofs,
            &mut circom_witness,
            &publics,
            &trace,
            &prover_buffer,
            output_dir_path.clone(),
        )?;
        timer_stop_and_log_info!(GENERATING_RECURSIVE2_PROOFS);
        info!("{}: Recursive2 proofs generated successfully", Self::MY_NAME);

        pctx.dctx.read().unwrap().barrier();
        if mpi_rank == 0 {
            let setup_final = setups.setup_vadcop_final.as_ref().unwrap().clone();
            timer_start_info!(GENERATING_VADCOP_FINAL_PROOF);
            let final_proof = generate_vadcop_final_proof(
                &pctx,
                setup_final.clone(),
                recursive2_proof,
                &mut circom_witness,
                &publics,
                &trace,
                &prover_buffer,
                output_dir_path.clone(),
            )?;
            timer_stop_and_log_info!(GENERATING_VADCOP_FINAL_PROOF);
            info!("{}: VadcopFinal proof generated successfully", Self::MY_NAME);

            timer_stop_and_log_info!(GENERATING_AGGREGATION_PROOFS);

            if pctx.options.final_snark {
                timer_start_info!(GENERATING_RECURSIVE_F_PROOF);
                let recursivef_proof = generate_recursivef_proof(
                    &pctx,
                    setups.setup_recursivef.as_ref().unwrap().clone(),
                    final_proof,
                    &mut circom_witness,
                    &publics,
                    &trace,
                    &prover_buffer,
                    output_dir_path.clone(),
                )?;
                timer_stop_and_log_info!(GENERATING_RECURSIVE_F_PROOF);

                timer_start_info!(GENERATING_FFLONK_SNARK_PROOF);
                let _ = generate_fflonk_snark_proof(&pctx, recursivef_proof, output_dir_path.clone());
                timer_stop_and_log_info!(GENERATING_FFLONK_SNARK_PROOF);
            } else {
                let setup_path = pctx.global_info.get_setup_path("vadcop_final");
                let stark_info_path = setup_path.display().to_string() + ".starkinfo.json";
                let expressions_bin_path = setup_path.display().to_string() + ".verifier.bin";
                let verkey_path = setup_path.display().to_string() + ".verkey.json";

                timer_start_info!(VERIFYING_VADCOP_FINAL_PROOF);
                valid_proofs = verify_proof(
                    final_proof,
                    stark_info_path,
                    expressions_bin_path,
                    verkey_path,
                    Some(pctx.get_publics().clone()),
                    None,
                    None,
                );
                timer_stop_and_log_info!(VERIFYING_VADCOP_FINAL_PROOF);
                if !valid_proofs {
                    log::info!(
                        "{}: ··· {}",
                        Self::MY_NAME,
                        "\u{2717} Vadcop Final proof was not verified".bright_red().bold()
                    );
                    return Err("Vadcop Final proof was not verified".into());
                } else {
                    log::info!(
                        "{}:     {}",
                        Self::MY_NAME,
                        "\u{2713} Vadcop Final proof was verified".bright_green().bold()
                    );
                }
            }
        }
        timer_stop_and_log_info!(GENERATING_VADCOP_PROOF);
        info!("{}: Proofs generated successfully", Self::MY_NAME);
        pctx.dctx.read().unwrap().barrier();
        Ok(())
    }

    fn initialize_witness(
        witness_lib_path: PathBuf,
        wcm: Arc<WitnessManager<F>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing witness");
        timer_start_info!(INITIALIZE_WITNESS);

        // Load the witness computation dynamic library
        let library = unsafe { Library::new(&witness_lib_path)? };

        let witness_lib: Symbol<WitnessLibInitFn<F>> = unsafe { library.get(b"init_library")? };
        let mut witness_lib = witness_lib(wcm.get_pctx().options.verbose_mode)?;
        witness_lib.register_witness(wcm.clone());

        wcm.start_proof();
        wcm.execute();

        timer_stop_and_log_info!(INITIALIZE_WITNESS);
        Ok(())
    }

    fn initialize_provers(sctx: Arc<SetupCtx>, provers: &mut Vec<Box<dyn Prover<F>>>, pctx: Arc<ProofCtx<F>>) {
        timer_start_debug!(INITIALIZE_PROVERS);
        let instances = pctx.dctx_get_instances();
        let my_instances = pctx.dctx_get_my_instances();
        for instance_id in my_instances.iter() {
            let (airgroup_id, air_id) = instances[*instance_id];
            let air_instance_id = pctx.dctx_find_air_instance_id(*instance_id);
            let (skip, constraints_skip) =
                skip_prover_instance(pctx.options.clone(), airgroup_id, air_id, air_instance_id);
            if skip {
                continue;
            };
            let prover = Box::new(StarkProver::new(
                sctx.clone(),
                airgroup_id,
                air_id,
                air_instance_id,
                *instance_id,
                constraints_skip,
            ));

            provers.push(prover);
        }

        for prover in provers.iter_mut() {
            prover.build(pctx.clone());
        }

        let mut buff_helper_size = 0_usize;

        for prover in provers.iter_mut() {
            let buff_helper_prover_size = prover.get_buff_helper_size(pctx.clone());
            if buff_helper_prover_size > buff_helper_size {
                buff_helper_size = buff_helper_prover_size;
            }
        }

        let buff_helper = create_buffer_fast(buff_helper_size);

        *pctx.buff_helper.values.write().unwrap() = buff_helper;
        timer_stop_and_log_debug!(INITIALIZE_PROVERS);
    }

    fn initialize_fixed_pols(setups: Arc<SetupsVadcop>, pctx: Arc<ProofCtx<F>>, save_file: bool) {
        info!("{}: Initializing setup fixed pols", Self::MY_NAME);
        timer_start_info!(INITIALIZE_CONST_POLS);

        let instances = pctx.dctx_get_instances();
        let my_instances = pctx.dctx_get_my_instances();

        let mut airs = Vec::new();
        let mut seen = HashSet::new();

        for instance_id in my_instances.iter() {
            let (airgroup_id, air_id) = instances[*instance_id];
            if seen.insert((airgroup_id, air_id)) {
                airs.push((airgroup_id, air_id));
            }
        }

        airs.iter().for_each(|&(airgroup_id, air_id)| {
            let setup = setups.sctx.get_setup(airgroup_id, air_id);
            setup.load_const_pols(&pctx.global_info, &ProofType::Basic);
            setup.load_const_pols_tree(&pctx.global_info, &ProofType::Basic, save_file);
        });

        timer_stop_and_log_info!(INITIALIZE_CONST_POLS);

        if pctx.options.aggregation {
            timer_start_info!(INITIALIZE_CONST_POLS_AGGREGATION);

            info!("{}: Initializing setup fixed pols aggregation", Self::MY_NAME);

            let global_info = pctx.global_info.clone();

            let sctx_compressor = setups.sctx_compressor.as_ref().unwrap().clone();
            info!("{}: ··· Initializing setup fixed pols compressor", Self::MY_NAME);
            timer_start_trace!(INITIALIZE_CONST_POLS_COMPRESSOR);

            airs.iter().for_each(|&(airgroup_id, air_id)| {
                if global_info.get_air_has_compressor(airgroup_id, air_id) {
                    let setup = sctx_compressor.get_setup(airgroup_id, air_id);
                    setup.load_const_pols(&global_info, &ProofType::Compressor);
                    setup.load_const_pols_tree(&global_info, &ProofType::Compressor, save_file);
                }
            });
            timer_stop_and_log_trace!(INITIALIZE_CONST_POLS_COMPRESSOR);

            let sctx_recursive1 = setups.sctx_recursive1.as_ref().unwrap().clone();
            timer_start_trace!(INITIALIZE_CONST_POLS_RECURSIVE1);
            info!("{}: ··· Initializing setup fixed pols recursive1", Self::MY_NAME);
            airs.iter().for_each(|&(airgroup_id, air_id)| {
                let setup = sctx_recursive1.get_setup(airgroup_id, air_id);
                setup.load_const_pols(&global_info, &ProofType::Recursive1);
                setup.load_const_pols_tree(&global_info, &ProofType::Recursive1, save_file);
            });
            timer_stop_and_log_trace!(INITIALIZE_CONST_POLS_RECURSIVE1);

            let sctx_recursive2 = setups.sctx_recursive2.as_ref().unwrap().clone();
            timer_start_trace!(INITIALIZE_CONST_POLS_RECURSIVE2);
            info!("{}: ··· Initializing setup fixed pols recursive2", Self::MY_NAME);
            let n_airgroups = global_info.air_groups.len();
            for airgroup in 0..n_airgroups {
                let setup = sctx_recursive2.get_setup(airgroup, 0);
                setup.load_const_pols(&global_info, &ProofType::Recursive2);
                setup.load_const_pols_tree(&global_info, &ProofType::Recursive2, save_file);
            }
            timer_stop_and_log_trace!(INITIALIZE_CONST_POLS_RECURSIVE2);

            if pctx.dctx_get_rank() == 0 {
                let setup_vadcop_final = setups.setup_vadcop_final.as_ref().unwrap().clone();
                timer_start_trace!(INITIALIZE_CONST_POLS_VADCOP_FINAL);
                info!("{}: ··· Initializing setup fixed pols vadcop final", Self::MY_NAME);
                setup_vadcop_final.load_const_pols(&global_info, &ProofType::VadcopFinal);
                setup_vadcop_final.load_const_pols_tree(&global_info, &ProofType::VadcopFinal, save_file);
                timer_stop_and_log_trace!(INITIALIZE_CONST_POLS_VADCOP_FINAL);

                if pctx.options.final_snark {
                    let global_info = pctx.global_info.clone();
                    let setup_recursivef = setups.setup_recursivef.as_ref().unwrap().clone();
                    timer_start_trace!(INITIALIZE_CONST_POLS_RECURSIVE_FINAL);
                    info!("{}: ··· Initializing setup fixed pols recursive final", Self::MY_NAME);
                    setup_recursivef.load_const_pols(&global_info, &ProofType::RecursiveF);
                    setup_recursivef.load_const_pols_tree(&global_info, &ProofType::RecursiveF, save_file);
                    timer_stop_and_log_trace!(INITIALIZE_CONST_POLS_RECURSIVE_FINAL);
                }
            }
            timer_stop_and_log_info!(INITIALIZE_CONST_POLS_AGGREGATION);
        }
    }

    pub fn calculate_stage(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        sctx: Arc<SetupCtx>,
        pctx: Arc<ProofCtx<F>>,
    ) {
        if stage as usize == pctx.global_info.n_challenges.len() + 1 {
            info!("{}: Calculating Quotient Polynomials", Self::MY_NAME);
            timer_start_info!(CALCULATING_QUOTIENT_POLYNOMIAL);
            for prover in provers.iter_mut() {
                prover.calculate_stage(stage, sctx.clone(), pctx.clone());
            }
            timer_stop_and_log_info!(CALCULATING_QUOTIENT_POLYNOMIAL);
        } else {
            info!("{}: Calculating stage {}", Self::MY_NAME, stage);
            timer_start_info!(CALCULATING_STAGE);
            for prover in provers.iter_mut() {
                prover.calculate_stage(stage, sctx.clone(), pctx.clone());
            }
            timer_stop_and_log_info!(CALCULATING_STAGE);
        }
    }

    pub fn commit_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>], pctx: Arc<ProofCtx<F>>) {
        if stage as usize == pctx.global_info.n_challenges.len() + 1 {
            info!("{}: Committing stage Q", Self::MY_NAME);
        } else {
            info!("{}: Committing stage {}", Self::MY_NAME, stage);
        }

        timer_start_info!(COMMITING_STAGE);
        for prover in provers.iter_mut() {
            prover.commit_stage(stage, pctx.clone());
        }
        timer_stop_and_log_info!(COMMITING_STAGE);
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
        transcript: &mut FFITranscript,
    ) {
        timer_start_debug!(CALCULATING_CHALLENGES);
        if pctx.options.verify_constraints {
            let dummy_elements = [F::zero(), F::one(), F::two(), F::neg_one()];
            transcript.add_elements(dummy_elements.as_ptr() as *mut u8, 4);
            return;
        }

        if stage == 1 {
            transcript.add_elements(pctx.get_publics_ptr(), pctx.global_info.n_publics);
        }

        let proof_values_stage = pctx.get_proof_values_by_stage(stage);
        if !proof_values_stage.is_empty() {
            transcript.add_elements(proof_values_stage.as_ptr() as *mut u8, proof_values_stage.len());
        }

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
        let all_roots = pctx.dctx_distribute_roots(roots);

        // add challenges to transcript in order
        for group_idxs in pctx.dctx_get_my_groups() {
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
                transcript.add_elements(value.as_ptr() as *mut u8, value.len());
            }
        }
        timer_stop_and_log_debug!(CALCULATING_CHALLENGES);
    }

    fn get_challenges(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        pctx: Arc<ProofCtx<F>>,
        transcript: &FFITranscript,
    ) {
        provers[0].get_challenges(stage, pctx, transcript); // Any prover can get the challenges which are common among them
    }

    pub fn opening_stages(
        provers: &mut [Box<dyn Prover<F>>],
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx>,

        transcript: &mut FFITranscript,
    ) {
        let num_commit_stages = pctx.global_info.n_challenges.len() as u32;

        // Calculate evals
        timer_start_debug!(CALCULATING_EVALS);
        Self::get_challenges(pctx.global_info.n_challenges.len() as u32 + 2, provers, pctx.clone(), transcript);
        for group_idx in pctx.dctx_get_my_air_groups() {
            provers[group_idx[0]].calculate_lev(pctx.clone());
            for idx in group_idx.iter() {
                provers[*idx].opening_stage(1, sctx.clone(), pctx.clone());
            }
        }

        timer_stop_and_log_debug!(CALCULATING_EVALS);
        Self::calculate_challenges(num_commit_stages + 2, provers, pctx.clone(), transcript);

        // Calculate fri polynomial
        Self::get_challenges(pctx.global_info.n_challenges.len() as u32 + 3, provers, pctx.clone(), transcript);
        info!("{}: Calculating FRI Polynomials", Self::MY_NAME);
        timer_start_info!(CALCULATING_FRI_POLINOMIAL);
        for group_idx in pctx.dctx_get_my_air_groups().iter() {
            provers[group_idx[0]].calculate_xdivxsub(pctx.clone());
            for idx in group_idx.iter() {
                provers[*idx].opening_stage(2, sctx.clone(), pctx.clone());
            }
        }
        timer_stop_and_log_info!(CALCULATING_FRI_POLINOMIAL);

        let global_steps_fri: Vec<usize> = pctx.global_info.steps_fri.iter().map(|step| step.n_bits).collect();
        let num_opening_stages = global_steps_fri.len() as u32;

        timer_start_info!(CALCULATING_FRI);
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
                    transcript,
                );
            }
            timer_stop_and_log_debug!(CALCULATING_FRI_STEP);
        }
        timer_stop_and_log_info!(CALCULATING_FRI);
    }

    fn get_proofs(provers: &mut [Box<dyn Prover<F>>], pctx: Arc<ProofCtx<F>>, output_dir: &str) -> Vec<*mut c_void> {
        timer_start_info!(GET_PROOFS);
        let mut proofs = Vec::new();

        for prover in provers.iter_mut() {
            let proof = prover.get_proof();
            proofs.push(proof);
        }

        let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
        let global_info_file: &str = global_info_path.to_str().unwrap();

        let publics_ptr = pctx.get_publics_ptr();
        let proof_values_ptr = pctx.get_proof_values_ptr();
        let challenges_ptr = pctx.get_challenges_ptr();

        let proves = vec![std::ptr::null_mut(); proofs.len()];

        let proofs_dir = match pctx.options.debug_info.save_proofs_to_file {
            true => output_dir,
            false => "",
        };

        fri_proof_get_zkinproofs_c(
            proofs.len() as u64,
            proves.as_ptr() as *mut *mut c_void,
            proofs.as_ptr() as *mut *mut c_void,
            publics_ptr,
            proof_values_ptr,
            challenges_ptr,
            global_info_file,
            proofs_dir,
        );

        timer_stop_and_log_info!(GET_PROOFS);

        if pctx.options.debug_info.save_proofs_to_file {
            let n_publics = pctx.global_info.n_publics as u64;

            save_publics_c(n_publics, pctx.get_publics_ptr(), output_dir);

            save_proof_values_c(pctx.get_proof_values_ptr(), global_info_file, output_dir);

            save_challenges_c(pctx.get_challenges_ptr(), global_info_file, output_dir);
        }

        proves
    }

    fn free_provers(provers: &mut [Box<dyn Prover<F>>], pctx: Arc<ProofCtx<F>>) {
        timer_start_info!(FREE_PROVERS);
        let mut proofs = Vec::new();
        let mut starks = Vec::new();

        for prover in provers.iter_mut() {
            proofs.push(prover.get_proof());
            starks.push(prover.get_stark());
        }

        let n_proofs = provers.len() as u64;

        free_provers_c(
            n_proofs,
            starks.as_ptr() as *mut *mut c_void,
            proofs.as_ptr() as *mut *mut c_void,
            pctx.options.aggregation,
        );

        if pctx.options.aggregation {
            std::thread::spawn(move || {
                pctx.free_instances();
            });
        } else {
            pctx.free_instances();
        }

        timer_stop_and_log_info!(FREE_PROVERS);
    }

    fn print_global_summary(pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        let mut air_info = HashMap::new();

        let mut air_instances = HashMap::new();

        let instances = pctx.dctx_get_instances();

        for (airgroup_id, air_id) in &instances {
            let air_name = pctx.global_info.airs[*airgroup_id][*air_id].clone().name;
            let air_group_name = pctx.global_info.air_groups[*airgroup_id].clone();
            let air_instance_map = air_instances.entry(air_group_name).or_insert_with(HashMap::new);
            if !air_instance_map.contains_key(&air_name.clone()) {
                let setup = sctx.get_setup(*airgroup_id, *air_id);
                let n_bits = setup.stark_info.stark_struct.n_bits;
                let memory_instance = setup.prover_buffer_size as f64 * 8.0;
                let memory_fixed = (setup.stark_info.n_constants * (1 << (setup.stark_info.stark_struct.n_bits))
                    + setup.stark_info.n_constants * (1 << (setup.stark_info.stark_struct.n_bits_ext)))
                    as f64
                    * 8.0;
                let memory_helpers = setup.stark_info.get_buff_helper_size() as f64 * 8.0;
                let total_cols: u64 = setup
                    .stark_info
                    .map_sections_n
                    .iter()
                    .filter(|(key, _)| *key != "const")
                    .map(|(_, value)| *value)
                    .sum();
                let cols_witness: u64 = setup.stark_info.map_sections_n["cm1"];
                air_info.insert(
                    air_name.clone(),
                    (n_bits, total_cols, cols_witness, memory_fixed, memory_helpers, memory_instance),
                );
            }
            let air_instance_map_key = air_instance_map.entry(air_name).or_insert(0);
            *air_instance_map_key += 1;
        }

        let mut air_groups: Vec<_> = air_instances.keys().collect();
        air_groups.sort();

        info!(
            "{}",
            format!("{}: --- TOTAL PROOF INSTANCES SUMMARY ------------------------", Self::MY_NAME)
                .bright_white()
                .bold()
        );
        info!("{}:     ► {} Air instances found:", Self::MY_NAME, instances.len());
        let mut total_cells = 0f64;
        let mut total_cells_witness = 0f64;
        for air_group in air_groups.clone() {
            let air_group_instances = air_instances.get(air_group).unwrap();
            let mut air_names: Vec<_> = air_group_instances.keys().collect();
            air_names.sort();

            info!("{}:       Air Group [{}]", Self::MY_NAME, air_group);
            for air_name in air_names {
                let count = air_group_instances.get(air_name).unwrap();
                let (n_bits, total_cols, cols_witness, _, _, _) = air_info.get(air_name).unwrap();
                total_cells += *total_cols as f64 * *count as f64 * (1 << *n_bits) as f64;
                total_cells_witness += *cols_witness as f64 * *count as f64 * (1 << *n_bits) as f64;
                info!(
                    "{}:       {}",
                    Self::MY_NAME,
                    format!("· {} x Air [{}] ({} x 2^{})", count, air_name, total_cols, n_bits).bright_white().bold()
                );
            }
        }
        info!("{} TOTAL CELLS WITNESS: {} and TOTAL CELLS {}", Self::MY_NAME, total_cells_witness, total_cells);
        info!("{}: ----------------------------------------------------------", Self::MY_NAME);
        info!(
            "{}",
            format!("{}: --- TOTAL PROVER MEMORY USAGE ----------------------------", Self::MY_NAME)
                .bright_white()
                .bold()
        );
        let mut total_memory = 0f64;
        let mut memory_helper_size = 0f64;
        for air_group in air_groups {
            let air_group_instances = air_instances.get(air_group).unwrap();
            let mut air_names: Vec<_> = air_group_instances.keys().collect();
            air_names.sort();

            for air_name in air_names {
                let count = air_group_instances.get(air_name).unwrap();
                let (_, _, _, memory_fixed, memory_helper_instance_size, memory_instance) =
                    air_info.get(air_name).unwrap();
                let total_memory_instance = memory_fixed + memory_instance * *count as f64;
                total_memory += total_memory_instance;
                if *memory_helper_instance_size > memory_helper_size {
                    memory_helper_size = *memory_helper_instance_size;
                }
                info!(
                    "{}:       {}",
                    Self::MY_NAME,
                    format!(
                        "· {}: {} fixed cols | {} per each of {} instance | Total {}",
                        air_name,
                        format_bytes(*memory_fixed),
                        format_bytes(*memory_instance),
                        count,
                        format_bytes(total_memory_instance)
                    )
                );
            }
        }
        total_memory += memory_helper_size;
        info!("{}:       {}", Self::MY_NAME, format!("Extra helper memory: {}", format_bytes(memory_helper_size)));
        info!(
            "{}:       {}",
            Self::MY_NAME,
            format!("Total prover memory required: {}", format_bytes(total_memory)).bright_white().bold()
        );
        info!("{}: ----------------------------------------------------------", Self::MY_NAME);
    }

    fn print_summary(pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        let mut air_info = HashMap::new();

        let mut air_instances = HashMap::new();

        let instances = pctx.dctx_get_instances();
        let my_instances = pctx.dctx_get_my_instances();

        for instance_id in my_instances.iter() {
            let (airgroup_id, air_id) = instances[*instance_id];
            let air_name = pctx.global_info.airs[airgroup_id][air_id].clone().name;
            let air_group_name = pctx.global_info.air_groups[airgroup_id].clone();
            let air_instance_map = air_instances.entry(air_group_name).or_insert_with(HashMap::new);
            if !air_instance_map.contains_key(&air_name.clone()) {
                let setup = sctx.get_setup(airgroup_id, air_id);
                let n_bits = setup.stark_info.stark_struct.n_bits;
                let memory_instance = setup.prover_buffer_size as f64 * 8.0;
                let memory_fixed = (setup.stark_info.n_constants * (1 << (setup.stark_info.stark_struct.n_bits))
                    + setup.stark_info.n_constants * (1 << (setup.stark_info.stark_struct.n_bits_ext)))
                    as f64
                    * 8.0;
                let memory_helpers = setup.stark_info.get_buff_helper_size() as f64 * 8.0;
                let total_cols: u64 = setup
                    .stark_info
                    .map_sections_n
                    .iter()
                    .filter(|(key, _)| *key != "const")
                    .map(|(_, value)| *value)
                    .sum();
                air_info.insert(air_name.clone(), (n_bits, total_cols, memory_fixed, memory_helpers, memory_instance));
            }
            let air_instance_map_key = air_instance_map.entry(air_name).or_insert(0);
            *air_instance_map_key += 1;
        }

        let mut air_groups: Vec<_> = air_instances.keys().collect();
        air_groups.sort();

        info!("{}: --- PROOF INSTANCES SUMMARY ------------------------", Self::MY_NAME);
        info!("{}:     ► {} Air instances found:", Self::MY_NAME, my_instances.len());
        for air_group in air_groups.clone() {
            let air_group_instances = air_instances.get(air_group).unwrap();
            let mut air_names: Vec<_> = air_group_instances.keys().collect();
            air_names.sort();

            info!("{}:       Air Group [{}]", Self::MY_NAME, air_group);
            for air_name in air_names {
                let count = air_group_instances.get(air_name).unwrap();
                let (n_bits, total_cols, _, _, _) = air_info.get(air_name).unwrap();
                info!(
                    "{}:       {}",
                    Self::MY_NAME,
                    format!("· {} x Air [{}] ({} x 2^{})", count, air_name, total_cols, n_bits).bright_white().bold()
                );
            }
        }
        info!("{}: ------------------------------------------------", Self::MY_NAME);
        info!("{}: --- PROVER MEMORY USAGE ------------------------", Self::MY_NAME);
        info!("{}:     ► {} Air instances found:", Self::MY_NAME, my_instances.len());
        let mut total_memory = 0f64;
        let mut memory_helper_size = 0f64;
        for air_group in air_groups {
            let air_group_instances = air_instances.get(air_group).unwrap();
            let mut air_names: Vec<_> = air_group_instances.keys().collect();
            air_names.sort();

            for air_name in air_names {
                let count = air_group_instances.get(air_name).unwrap();
                let (_, _, memory_fixed, memory_helper_instance_size, memory_instance) =
                    air_info.get(air_name).unwrap();
                let total_memory_instance = memory_fixed + memory_instance * *count as f64;
                total_memory += total_memory_instance;
                if *memory_helper_instance_size > memory_helper_size {
                    memory_helper_size = *memory_helper_instance_size;
                }
                info!(
                    "{}:       {}",
                    Self::MY_NAME,
                    format!(
                        "· {}: {} fixed cols | {} per each of {} instance | Total {}",
                        air_name,
                        format_bytes(*memory_fixed),
                        format_bytes(*memory_instance),
                        count,
                        format_bytes(total_memory_instance)
                    )
                );
            }
        }
        total_memory += memory_helper_size;
        info!("{}:       {}", Self::MY_NAME, format!("Extra helper memory: {}", format_bytes(memory_helper_size)));
        info!(
            "{}:       {}",
            Self::MY_NAME,
            format!("Total prover memory required: {}", format_bytes(total_memory)).bright_white().bold()
        );
        info!("{}: ------------------------------------------------", Self::MY_NAME);
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
