use libloading::{Library, Symbol};
use log::{debug, info, trace};
use p3_field::Field;
use stark::{StarkBufferAllocator, StarkProver};
use starks_lib_c::verify_global_constraints_c;

use proofman_setup::SetupCtx;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use transcript::FFITranscript;

use crate::{WitnessLibrary, WitnessLibInitFn};

use proofman_common::{Prover, ExecutionCtx, ProofCtx};

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
        debug_mode: bool,
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

        // Load the witness computation dynamic library
        let library = unsafe { Library::new(&witness_lib_path)? };

        let witness_lib: Symbol<WitnessLibInitFn<F>> = unsafe { library.get(b"init_library")? };

        let mut witness_lib = witness_lib(rom_path.clone(), public_inputs_path.clone())?;

        let mut pctx = ProofCtx::create_ctx(witness_lib.pilout());

        let mut provers: Vec<Box<dyn Prover<F>>> = Vec::new();

        let buffer_allocator: Arc<StarkBufferAllocator> = Arc::new(StarkBufferAllocator::new(proving_key_path.clone()));

        let mut ectx = ExecutionCtx::builder().with_buffer_allocator(buffer_allocator).build();

        let sctx = SetupCtx::new(witness_lib.pilout(), &proving_key_path);

        Self::initialize_witness(&mut witness_lib, &mut pctx, &mut ectx, &sctx);

        witness_lib.calculate_witness(1, &mut pctx, &ectx, &sctx);

        Self::initialize_provers(&sctx, &proving_key_path, &mut provers, &mut pctx);

        if provers.is_empty() {
            return Err("No instances found".into());
        }
        let mut transcript = provers[0].new_transcript();

        Self::calculate_challenges(0, &mut provers, &mut pctx, &mut transcript, false);
        provers[0].add_publics_to_transcript(&mut pctx, &transcript);

        let mut valid_constraints = true;

        // Commit stages
        let num_commit_stages = pctx.pilout.num_stages();
        for stage in 1..=num_commit_stages {
            if stage != 1 {
                witness_lib.calculate_witness(stage, &mut pctx, &ectx, &sctx);
            }

            Self::get_challenges(stage, &mut provers, &mut pctx, &transcript);

            Self::calculate_stage(stage, &mut provers, &mut pctx);

            if debug_mode {
                valid_constraints = Self::verify_constraints(stage, &mut provers);
            } else {
                Self::commit_stage(stage, &mut provers);
            }

            Self::calculate_challenges(stage, &mut provers, &mut pctx, &mut transcript, debug_mode);
        }

        witness_lib.end_proof();

        if debug_mode {
            let mut proofs: Vec<*mut c_void> = Vec::new();

            for prover in provers.iter_mut() {
                let proof = prover.get_proof();
                proofs.push(proof);
            }

            valid_constraints = verify_global_constraints_c(
                &proving_key_path.join("pilout.globalInfo.json").to_str().unwrap(),
                &proving_key_path.join("pilout.globalConstraints.bin").to_str().unwrap(),
                pctx.public_inputs.as_ptr() as *mut c_void,
                proofs.as_mut_ptr() as *mut c_void,
                provers.len() as u64,
            );

            if !valid_constraints {
                log::debug!("{}", "Not all constraints were verified.".bright_red().bold());
            } else {
                log::debug!("{}", "All constraints were successfully verified.".bright_green().bold());
            }

            return Ok(vec![]);
        }

        // Compute Quotient polynomial
        Self::get_challenges(pctx.pilout.num_stages() + 1, &mut provers, &mut pctx, &transcript);
        Self::calculate_stage(pctx.pilout.num_stages() + 1, &mut provers, &mut pctx);
        Self::commit_stage(pctx.pilout.num_stages() + 1, &mut provers);
        Self::calculate_challenges(pctx.pilout.num_stages() + 1, &mut provers, &mut pctx, &mut transcript, false);

        // Compute openings
        Self::opening_stages(&mut provers, &mut pctx, &mut transcript);

        let proof = Self::finalize_proof(&pctx);

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

        for air_instance in pctx.air_instances.write().unwrap().iter_mut() {
            debug!(
                "{}: Initializing prover for air instance ({}, {})",
                Self::MY_NAME,
                air_instance.air_group_id,
                air_instance.air_id
            );

            let prover =
                Box::new(StarkProver::new(sctx, proving_key_path, air_instance.air_group_id, air_instance.air_id));

            provers.push(prover);
        }
        for (idx, prover) in provers.iter_mut().enumerate() {
            prover.build(pctx, idx);
        }
    }

    pub fn verify_constraints(stage: u32, provers: &mut [Box<dyn Prover<F>>]) -> bool {
        info!("{}: Verifying constraints stage {}", Self::MY_NAME, stage);

        let mut valid_constraints = true;
        for (idx, prover) in provers.iter_mut().enumerate() {
            info!("{}: Verifying constraints stage {}, for prover {}", Self::MY_NAME, stage, idx);
            valid_constraints = prover.verify_constraints(stage);
        }
        valid_constraints
    }

    pub fn calculate_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>], pctx: &mut ProofCtx<F>) {
        info!("{}: Calculating stage {}", Self::MY_NAME, stage);
        for (idx, prover) in provers.iter_mut().enumerate() {
            info!("{}: Calculating stage {}, for prover {}", Self::MY_NAME, stage, idx);
            prover.calculate_stage(stage, pctx);
        }
    }

    pub fn commit_stage(stage: u32, provers: &mut [Box<dyn Prover<F>>]) {
        info!("{}: Committing stage {}", Self::MY_NAME, stage);

        for (idx, prover) in provers.iter_mut().enumerate() {
            info!("{}: Committing stage {}, for prover {}", Self::MY_NAME, stage, idx);
            prover.commit_stage(stage);
        }
    }

    fn calculate_challenges(
        stage: u32,
        provers: &mut [Box<dyn Prover<F>>],
        pctx: &mut ProofCtx<F>,
        transcript: &mut FFITranscript,
        debug_mode: bool,
    ) {
        info!("{}: Calculating challenges for stage {}", Self::MY_NAME, stage);
        for prover in provers.iter_mut() {
            if debug_mode {
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
            Self::calculate_challenges(pctx.pilout.num_stages() + 1 + opening_id, provers, pctx, transcript, false);
        }
    }

    fn finalize_proof(_proof_ctx: &ProofCtx<F>) -> Vec<F> {
        // This is a mock implementation
        vec![]
    }
}
