use std::error::Error;
use std::path::{Path, PathBuf};

use std::any::type_name;

use proofman_common::{
    BufferAllocator, ConstraintInfo, ConstraintsResults, GlobalInfo, ProofCtx, Prover, ProverInfo, ProverStatus,
    SetupCtx,
};
use log::{debug, trace};
use transcript::FFITranscript;
use proofman_util::{timer_start, timer_stop_and_log};
use proofman_starks_lib_c::*;
use crate::stark_info::StarkInfo;
use crate::stark_prover_settings::StarkProverSettings;
use p3_goldilocks::Goldilocks;
use p3_field::Field;

use std::os::raw::c_void;

#[repr(C)]
pub struct VecU64Result {
    pub n_elements: u64,
    pub ids: *mut u64,
}

#[allow(dead_code)]
pub struct StarkProver<T: Field> {
    initialized: bool,
    prover_idx: usize,
    air_id: usize,
    air_group_id: usize,
    config: StarkProverSettings,
    p_setup: *mut c_void,
    pub p_stark: *mut c_void,
    p_stark_info: *mut c_void,
    stark_info: StarkInfo,
    n_field_elements: usize,
    merkle_tree_arity: u64,
    merkle_tree_custom: bool,
    p_proof: Option<*mut c_void>,
    evals: Vec<T>,
    global_steps_fri: Vec<usize>,
    pub subproof_values: Vec<T>,
}

impl<T: Field> StarkProver<T> {
    const MY_NAME: &'static str = "estrkPrv";

    const HASH_SIZE: usize = 4;
    const FIELD_EXTENSION: usize = 3;

    pub fn new(
        sctx: &SetupCtx,
        proving_key_path: &Path,
        air_group_id: usize,
        air_id: usize,
        prover_idx: usize,
    ) -> Self {
        let global_info = GlobalInfo::from_file(&proving_key_path.join("pilout.globalInfo.json"));

        let air_setup_folder = proving_key_path.join(global_info.get_air_setup_path(air_group_id, air_id));
        trace!("{}   : ··· Setup AIR folder: {:?}", Self::MY_NAME, air_setup_folder);

        // Check path exists and is a folder
        if !air_setup_folder.exists() {
            panic!("Setup AIR folder not found at path: {:?}", air_setup_folder);
        }
        if !air_setup_folder.is_dir() {
            panic!("Setup AIR path is not a folder: {:?}", air_setup_folder);
        }

        let base_filename_path =
            air_setup_folder.join(global_info.get_air_name(air_group_id, air_id)).display().to_string();

        let setup = sctx.get_setup(air_group_id, air_id).expect("REASON");

        let p_setup = setup.p_setup;
        let p_stark_info = setup.p_stark_info;

        let p_stark = starks_new_c(p_setup);

        let stark_info_path = base_filename_path.clone() + ".starkinfo.json";
        let stark_info_json = std::fs::read_to_string(&stark_info_path)
            .unwrap_or_else(|_| panic!("Failed to read file {}", &stark_info_path));
        let stark_info: StarkInfo = StarkInfo::from_json(&stark_info_json);

        let config = StarkProverSettings {
            current_path: air_setup_folder.to_str().unwrap().to_string(),
            stark_info_filename: stark_info_path,
            verkey_filename: base_filename_path.clone() + ".verkey.json",
        };

        let n_field_elements;
        let merkle_tree_arity;
        let merkle_tree_custom;

        if stark_info.stark_struct.verification_hash_type == "BN128" {
            n_field_elements = 1;
            merkle_tree_arity = stark_info.stark_struct.merkle_tree_arity;
            merkle_tree_custom = stark_info.stark_struct.merkle_tree_custom;
        } else {
            n_field_elements = Self::HASH_SIZE;
            merkle_tree_arity = 2;
            merkle_tree_custom = true;
        }

        let global_steps_fri: Vec<usize> = global_info.steps_fri.iter().map(|step| step.n_bits).collect();

        Self {
            initialized: true,
            prover_idx,
            air_id,
            air_group_id,
            config,
            p_setup,
            p_stark_info,
            p_stark,
            p_proof: None,
            stark_info,
            n_field_elements,
            merkle_tree_arity,
            merkle_tree_custom,
            evals: Vec::new(),
            global_steps_fri,
            subproof_values: Vec::new(),
        }
    }
}

impl<F: Field> Prover<F> for StarkProver<F> {
    fn build(&mut self, proof_ctx: &mut ProofCtx<F>) {
        timer_start!(ESTARK_PROVER_BUILD);
        let air_instance_ctx = &mut proof_ctx.air_instances.write().unwrap()[self.prover_idx];

        let ptr: *mut std::ffi::c_void = air_instance_ctx.get_buffer_ptr() as *mut c_void;

        //initialize the common challenges if have not been initialized by another prover
        proof_ctx.challenges.get_or_insert_with(|| {
            vec![F::zero(); self.stark_info.challenges_map.as_ref().unwrap().len() * Self::FIELD_EXTENSION]
        });

        self.evals = vec![F::zero(); self.stark_info.ev_map.len() * Self::FIELD_EXTENSION];

        self.subproof_values = vec![F::zero(); self.stark_info.n_subproof_values as usize * Self::FIELD_EXTENSION];

        let p_params = init_params_c(
            ptr,
            proof_ctx.public_inputs.as_ptr() as *mut c_void,
            proof_ctx.challenges.as_ref().unwrap().as_ptr() as *mut c_void,
            self.evals.as_ptr() as *mut c_void,
            self.subproof_values.as_ptr() as *mut c_void,
        );

        air_instance_ctx.set_params(p_params);

        let n_commits = self.stark_info.cm_pols_map.as_ref().expect("REASON").len();
        let n_subproof_values = self.stark_info.subproofvalues_map.as_ref().expect("REASON").len();
        air_instance_ctx.init_vec(n_commits, n_subproof_values);

        self.p_proof = Some(fri_proof_new_c(self.p_setup));

        let number_stage1_commits = *self.stark_info.map_sections_n.get("cm1").unwrap() as usize;
        for i in 0..number_stage1_commits {
            air_instance_ctx.set_commit_calculated(i);
        }

        self.initialized = true;

        timer_stop_and_log!(ESTARK_PROVER_BUILD);
    }

    fn new_transcript(&self) -> FFITranscript {
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        //initialize the transcript if has not been initialized by another prover
        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };
        FFITranscript::new(p_stark, element_type, self.merkle_tree_arity, self.merkle_tree_custom)
    }

    fn num_stages(&self) -> u32 {
        self.stark_info.n_stages
    }

    fn num_opening_stages(&self) -> u32 {
        self.global_steps_fri.len() as u32 + 3 //evals + fri_pol + fri_folding (steps) + fri_queries
    }

    fn verify_constraints(&self, proof_ctx: &mut ProofCtx<F>) -> Vec<ConstraintInfo> {
        let air_instance_ctx = &mut proof_ctx.air_instances.write().unwrap()[self.prover_idx];

        let raw_ptr = verify_constraints_c(self.p_setup, air_instance_ctx.params.unwrap());

        let constraints_result = unsafe { Box::from_raw(raw_ptr as *mut ConstraintsResults) };

        unsafe {
            std::slice::from_raw_parts(constraints_result.constraints_info, constraints_result.n_constraints as usize)
        }
        .to_vec()
    }

    fn calculate_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<F>) {
        let air_instance_ctx = &mut proof_ctx.air_instances.write().unwrap()[self.prover_idx];

        let n_commits = self.stark_info.cm_pols_map.as_ref().expect("REASON").len();

        if stage_id <= proof_ctx.pilout.num_stages() {
            for i in 0..n_commits {
                let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                if (cm_pol.stage < stage_id as u64 || cm_pol.stage == stage_id as u64 && !cm_pol.im_pol)
                    && !air_instance_ctx.commits_calculated[i]
                {
                    panic!("Intermediate polynomials for stage {} cannot be calculated: Witness column {} is not calculated", stage_id, cm_pol.name);
                }
            }
            calculate_impols_expressions_c(self.p_stark, air_instance_ctx.params.unwrap(), stage_id as u64);
            for i in 0..n_commits {
                let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                if cm_pol.stage == stage_id as u64 && cm_pol.im_pol {
                    air_instance_ctx.set_commit_calculated(i);
                }
            }
            if stage_id == proof_ctx.pilout.num_stages() {
                let p_proof = self.p_proof.unwrap();
                fri_proof_set_subproof_values_c(p_proof, air_instance_ctx.params.unwrap());
            }
        } else {
            calculate_quotient_polynomial_c(self.p_stark, air_instance_ctx.params.unwrap());
            for i in 0..n_commits {
                let cm_pol: &crate::stark_info::PolMap =
                    self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                if cm_pol.stage == (proof_ctx.pilout.num_stages() + 1) as u64 {
                    air_instance_ctx.set_commit_calculated(i);
                }
            }
        }
    }

    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<F>) -> ProverStatus {
        let air_instance_ctx = &mut proof_ctx.air_instances.write().unwrap()[self.prover_idx];
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        debug!("{}: ··· Computing commit stage {}", Self::MY_NAME, stage_id);

        timer_start!(STARK_COMMIT_STAGE_, stage_id);

        let p_proof = self.p_proof.unwrap();
        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };

        let n_commits = self.stark_info.cm_pols_map.as_ref().expect("REASON").len();
        for i in 0..n_commits {
            let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
            if cm_pol.stage == stage_id as u64 && !air_instance_ctx.commits_calculated[i] {
                panic!("Stage {} cannot be committed: Witness column {} is not calculated", stage_id, cm_pol.name);
            }
        }

        if stage_id == self.num_stages() {
            let n_subproof_values = self.stark_info.subproofvalues_map.as_ref().expect("REASON").len();
            for i in 0..n_subproof_values {
                let subproof_value = self.stark_info.subproofvalues_map.as_ref().expect("REASON").get(i).unwrap();
                if !air_instance_ctx.subproofvalue_calculated[i] {
                    panic!(
                        "Stage {} cannot be committed: Subproofvalue {} is not calculated",
                        stage_id, subproof_value.name
                    );
                }
            }
        }

        commit_stage_c(p_stark, element_type, stage_id as u64, air_instance_ctx.params.unwrap(), p_proof);

        timer_stop_and_log!(STARK_COMMIT_STAGE_, stage_id);

        if stage_id <= self.num_stages() + 1 {
            ProverStatus::CommitStage
        } else {
            ProverStatus::OpeningStage
        }
    }

    fn opening_stage(
        &mut self,
        opening_id: u32,
        proof_ctx: &mut ProofCtx<F>,
        transcript: &mut FFITranscript,
    ) -> ProverStatus {
        let last_stage_id = self.num_opening_stages();
        if opening_id == 1 {
            self.compute_evals(opening_id, proof_ctx);
        } else if opening_id == 2 {
            self.compute_fri_pol(opening_id, proof_ctx);
        } else if opening_id < last_stage_id {
            let global_step_fri = self.global_steps_fri[(opening_id - 3) as usize];
            let step_index =
                self.stark_info.stark_struct.steps.iter().position(|s| s.n_bits as usize == global_step_fri);
            if let Some(step_index) = step_index {
                self.compute_fri_folding(step_index as u32, proof_ctx, transcript);
            } else {
                debug!("{}: ··· Skipping FRI Folding", Self::MY_NAME,);
                transcript.add_elements(
                    [F::zero(), F::zero(), F::zero(), F::zero()].as_ptr() as *mut c_void,
                    self.n_field_elements,
                );
            }
        } else if opening_id == last_stage_id {
            self.compute_fri_queries(opening_id, proof_ctx);
        } else {
            panic!("Opening stage not implemented");
        }

        if opening_id == last_stage_id {
            ProverStatus::StagesCompleted
        } else {
            ProverStatus::OpeningStage
        }
    }

    fn add_challenges_to_transcript(&self, stage: u64, _proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript) {
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        if stage <= (Self::num_stages(self) + 1) as u64 {
            let root = vec![F::zero(); self.n_field_elements];

            let tree_index = if stage == 0 {
                let stark_info: &StarkInfo = &self.stark_info;
                stark_info.n_stages as u64 + 1
            } else {
                stage - 1
            };

            treesGL_get_root_c(p_stark, tree_index, root.as_ptr() as *mut c_void);

            transcript.add_elements(root.as_ptr() as *mut c_void, self.n_field_elements);
        } else if stage == (Self::num_stages(self) + 2) as u64 {
            //TODO: hardcoded, option no hash must be included
            let hash: Vec<F> = vec![F::zero(); self.n_field_elements];
            calculate_hash_c(
                p_stark,
                hash.as_ptr() as *mut c_void,
                self.evals.as_ptr() as *mut c_void,
                (self.stark_info.ev_map.len() * Self::FIELD_EXTENSION) as u64,
            );
            transcript.add_elements(hash.as_ptr() as *mut c_void, self.n_field_elements);
        }
    }

    //TODO: This funciton could leave outside the prover trait, for now is confortable to get the hash and the configs
    fn add_publics_to_transcript(&self, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript) {
        let stark_info: &StarkInfo = &self.stark_info;
        transcript.add_elements(proof_ctx.public_inputs.as_mut_ptr() as *mut c_void, stark_info.n_publics as usize);
    }

    fn get_challenges(&self, stage_id: u32, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript) {
        if stage_id == 1 {
            return;
        }

        if stage_id <= self.num_stages() + 3 {
            //num stages + 1 + evals + fri_pol (then starts fri folding...)

            let challenges_map = self.stark_info.challenges_map.as_ref().unwrap();

            let challenges: &Vec<F> = proof_ctx.challenges.as_ref().unwrap();
            for i in 0..challenges_map.len() {
                if challenges_map[i].stage == stage_id as u64 {
                    transcript.get_challenge(&challenges[i * Self::FIELD_EXTENSION] as *const F as *mut c_void);
                }
            }
        } else {
            //Fri folding + . queries: add one challenge for each step
            proof_ctx.challenges.as_mut().unwrap().extend(std::iter::repeat(F::zero()).take(3));
            let challenges: &Vec<F> = proof_ctx.challenges.as_ref().unwrap();
            transcript.get_challenge(&challenges[challenges.len() - 3] as *const F as *mut c_void);
        }
    }

    fn get_proof(&self) -> *mut c_void {
        self.p_proof.unwrap()
    }

    fn save_proof(&self, id: u64, output_dir: &str) {
        save_proof_c(id, self.p_stark_info, self.p_proof.unwrap(), output_dir);
    }

    fn get_prover_info(&self) -> ProverInfo {
        ProverInfo { air_group_id: self.air_group_id, air_id: self.air_id, prover_idx: self.prover_idx }
    }
}

impl<F: Field> StarkProver<F> {
    // Return the total number of elements needed to compute the STARK
    pub fn get_total_bytes(&self) -> usize {
        get_map_totaln_c(self.p_setup) as usize * std::mem::size_of::<F>()
    }

    fn compute_evals(&mut self, _opening_id: u32, proof_ctx: &mut ProofCtx<F>) {
        let air_instance_ctx = &mut proof_ctx.air_instances.write().unwrap()[self.prover_idx];

        let p_stark = self.p_stark;
        let p_proof = self.p_proof.unwrap();

        debug!("{}: ··· Computing evaluations", Self::MY_NAME);

        compute_evals_c(p_stark, air_instance_ctx.params.unwrap(), p_proof);
    }

    fn compute_fri_pol(&mut self, _opening_id: u32, proof_ctx: &mut ProofCtx<F>) {
        let air_instance_ctx = &mut proof_ctx.air_instances.write().unwrap()[self.prover_idx];

        let p_stark = self.p_stark;

        debug!("{}: ··· Computing FRI Polynomial", Self::MY_NAME);

        prepare_fri_polynomial_c(p_stark, air_instance_ctx.params.unwrap());

        calculate_fri_polynomial_c(p_stark, air_instance_ctx.params.unwrap());
    }

    fn compute_fri_folding(&mut self, step_index: u32, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript) {
        let air_instance_ctx = &mut proof_ctx.air_instances.write().unwrap()[self.prover_idx];
        let p_stark = self.p_stark;
        let p_proof = self.p_proof.unwrap();

        let steps = &self.stark_info.stark_struct.steps;
        let n_steps = (steps.len() - 1) as u32;
        if step_index == n_steps {
            debug!("{}: ··· Computing FRI folding for last step {}", Self::MY_NAME, steps[step_index as usize].n_bits,);
        } else {
            debug!(
                "{}: ··· Computing FRI folding from {} to {}",
                Self::MY_NAME,
                steps[step_index as usize].n_bits,
                steps[(step_index + 1) as usize].n_bits
            );
        }

        let challenges: &Vec<F> = proof_ctx.challenges.as_ref().unwrap();
        let challenge: Vec<F> = challenges.iter().skip(challenges.len() - 3).cloned().collect();

        compute_fri_folding_c(
            p_stark,
            step_index as u64,
            air_instance_ctx.params.unwrap(),
            challenge.as_ptr() as *mut c_void,
            p_proof,
        );

        if step_index < n_steps {
            let root = fri_proof_get_tree_root_c(p_proof, (step_index + 1) as u64, 0);
            transcript.add_elements(root, self.n_field_elements);
        } else {
            let hash: Vec<F> = vec![F::zero(); self.n_field_elements];
            let n_hash = (1 << (steps[n_steps as usize].n_bits)) * Self::FIELD_EXTENSION as u64;
            let fri_pol = get_fri_pol_c(self.p_setup, air_instance_ctx.params.unwrap());
            calculate_hash_c(p_stark, hash.as_ptr() as *mut c_void, fri_pol, n_hash);
            transcript.add_elements(hash.as_ptr() as *mut c_void, self.n_field_elements);
        }
    }

    fn compute_fri_queries(&mut self, _opening_id: u32, proof_ctx: &mut ProofCtx<F>) {
        let p_stark = self.p_stark;
        let p_proof = self.p_proof.unwrap();

        debug!("{}: ··· Computing FRI queries", Self::MY_NAME);

        let mut fri_queries = vec![u64::default(); self.stark_info.stark_struct.n_queries as usize];

        let challenges: &Vec<F> = proof_ctx.challenges.as_ref().unwrap();
        let challenge: Vec<F> = challenges.iter().skip(challenges.len() - 3).cloned().collect();

        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };
        let transcript_permutation =
            FFITranscript::new(p_stark, element_type, self.merkle_tree_arity, self.merkle_tree_custom);

        transcript_permutation.add_elements(challenge.as_ptr() as *mut c_void, Self::FIELD_EXTENSION);
        transcript_permutation.get_permutations(
            fri_queries.as_mut_ptr(),
            self.stark_info.stark_struct.n_queries,
            self.stark_info.stark_struct.steps[0].n_bits,
        );

        compute_fri_queries_c(p_stark, p_proof, fri_queries.as_mut_ptr());
    }
}

pub struct StarkBufferAllocator {
    pub proving_key_path: PathBuf,
}

impl StarkBufferAllocator {
    pub fn new(proving_key_path: PathBuf) -> Self {
        Self { proving_key_path }
    }
}

impl BufferAllocator for StarkBufferAllocator {
    fn get_buffer_info(&self, air_name: String, air_id: usize) -> Result<(u64, Vec<u64>), Box<dyn Error>> {
        let global_info_name = GlobalInfo::from_file(&self.proving_key_path.join("pilout.globalInfo.json")).name;

        // Get inside the proving key folder the unique file ending with "starkinfo.json", if not error
        let mut stark_info_path = None;

        let air_pk_folder = self
            .proving_key_path
            .join(global_info_name)
            .join(air_name.clone())
            .join("airs")
            .join(format!("{}_{}", air_name, air_id))
            .join("air");

        if !air_pk_folder.exists() {
            return Err(format!("The path does not exist: {:?}", air_pk_folder).into());
        }

        if !air_pk_folder.is_dir() {
            return Err("The path is not a directory".into());
        }

        for entry in std::fs::read_dir(air_pk_folder)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        if file_name_str.ends_with("starkinfo.json") {
                            stark_info_path = Some(path);
                            break;
                        }
                    }
                }
            }
        }

        if stark_info_path.is_none() {
            return Err("The path does not contain a file with extension 'starkinfo.json'".into());
        }

        let p_stark_info = stark_info_new_c(stark_info_path.unwrap().to_str().unwrap());

        Ok((get_map_totaln_c(p_stark_info), vec![get_map_offsets_c(p_stark_info, "cm1", false)]))
    }
}
