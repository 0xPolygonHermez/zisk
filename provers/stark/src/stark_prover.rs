use std::error::Error;
use std::path::{Path, PathBuf};

use std::any::type_name;

// use proofman_hints::{get_hint_ids_by_name, get_hint_field, set_hint_field, set_hint_field_val};
use proofman_common::{BufferAllocator, Prover, ProverStatus, ProofCtx};
use log::{debug, trace};
use proofman_setup::{GlobalInfo, SetupCtx};
use transcript::FFITranscript;
use proofman_util::{timer_start, timer_stop_and_log};
use starks_lib_c::*;
use crate::stark_info::StarkInfo;
use crate::stark_prover_settings::StarkProverSettings;
use p3_goldilocks::Goldilocks;
use p3_field::Field;

use std::os::raw::c_void;

#[allow(dead_code)]
pub struct StarkProver<T: Field> {
    initialized: bool,
    config: StarkProverSettings,
    p_steps: *mut c_void,
    pub p_stark: *mut c_void,
    stark_info: StarkInfo,
    p_starkinfo: *mut c_void,
    n_field_elements: usize,
    merkle_tree_arity: u64,
    merkle_tree_custom: bool,
    p_proof: Option<*mut c_void>,
    evals: Vec<T>,
    pub subproof_values: Vec<T>,
}

impl<T: Field> StarkProver<T> {
    const MY_NAME: &'static str = "estrkPrv";

    const HASH_SIZE: usize = 4;
    const FIELD_EXTENSION: usize = 3;

    pub fn new(sctx: &SetupCtx, proving_key_path: &Path, air_group_id: usize, air_id: usize) -> Self {
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

        let stark_info_path = base_filename_path.clone() + ".starkinfo.json";

        let p_starkinfo = stark_info_new_c(&stark_info_path);

        let p_steps = sctx.get_setup(air_group_id, air_id).expect("REASON");

        let p_stark = starks_new_default_c(p_starkinfo, p_steps);

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

        Self {
            initialized: true,
            config,
            p_steps,
            p_stark,
            p_proof: None,
            stark_info,
            p_starkinfo,
            n_field_elements,
            merkle_tree_arity,
            merkle_tree_custom,
            evals: Vec::new(),
            subproof_values: Vec::new(),
        }
    }

    pub fn get_stark_info(&self) -> *mut c_void {
        get_stark_info_c(self.p_stark)
    }
}

impl<F: Field> Prover<F> for StarkProver<F> {
    fn build(&mut self, proof_ctx: &mut ProofCtx<F>, air_idx: usize) {
        timer_start!(ESTARK_PROVER_BUILD);
        let air_instance_ctx = &mut proof_ctx.air_instances.write().unwrap()[air_idx];

        let ptr: *mut std::ffi::c_void = air_instance_ctx.get_buffer_ptr() as *mut c_void;

        //initialize the common challenges if have not been initialized by another prover
        proof_ctx.challenges.get_or_insert_with(|| {
            vec![F::zero(); self.stark_info.challenges_map.as_ref().unwrap().len() * Self::FIELD_EXTENSION]
        });

        self.evals = vec![F::zero(); self.stark_info.ev_map.len() * Self::FIELD_EXTENSION];

        self.subproof_values = vec![F::zero(); self.stark_info.n_subproof_values as usize * Self::FIELD_EXTENSION];

        init_params_c(
            self.p_steps,
            proof_ctx.challenges.as_ref().unwrap().as_ptr() as *mut c_void,
            self.subproof_values.as_ptr() as *mut c_void,
            self.evals.as_ptr() as *mut c_void,
            proof_ctx.public_inputs.as_ptr() as *mut c_void,
        );

        set_trace_pointer_c(self.p_steps, ptr);

        self.p_proof = Some(fri_proof_new_c(self.p_stark));

        let number_stage1_commits = *self.stark_info.map_sections_n.get("cm1").unwrap() as usize;
        for i in 0..number_stage1_commits {
            set_commit_calculated_c(self.p_steps, i as u64);
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
        self.stark_info.stark_struct.steps.len() as u32 + 3 //evals + fri_pol + fri_folding (setps) + fri_queries
    }

    fn verify_constraints(&self, stage_id: u32) -> bool {
        debug!("{}: ··· Verifying constraints for stage {}", Self::MY_NAME, stage_id);

        let p_steps = self.p_steps;
        verify_constraints_c(p_steps, stage_id as u64)
    }

    fn calculate_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<F>) {
        let p_steps = self.p_steps;

        // // THIS IS AN EXAMPLE OF HOW TO USE HINT FUNCTIONS
        // if stage_id == 2 {
        //     let stark_info: &StarkInfo = &self.stark_info;
        //     let n = 1 << stark_info.stark_struct.n_bits;

        //     let hints = get_hint_ids_by_name(p_steps, "gprod_col");

        //     for hint_id in hints.iter() {
        //         let num = get_hint_field::<F>(p_steps, *hint_id as usize, "numerator", false);
        //         let den = get_hint_field::<F>(p_steps, *hint_id as usize, "denominator", false);

        //         let mut reference = get_hint_field::<F>(p_steps, *hint_id as usize, "reference", true);

        //         reference.set(0, num.get(0) / den.get(0));
        //         for i in 1..n {
        //             reference.set(i, reference.get(i - 1) * (num.get(i) / den.get(i)));
        //         }

        //         set_hint_field_val(p_steps, 0, "result", reference.get(n -1));

        //         set_hint_field(p_steps, 0, "reference", &reference);
        //     }
        // }

        if stage_id <= proof_ctx.pilout.num_stages() {
            can_impols_be_calculated_c(p_steps, stage_id as u64);
            calculate_impols_expressions_c(p_steps, stage_id as u64);
            if stage_id == proof_ctx.pilout.num_stages() {
                let p_proof = self.p_proof.unwrap();
                fri_proof_set_subproof_values_c(p_proof, p_steps);
            }
        } else {
            calculate_quotient_polynomial_c(p_steps);
        }
    }

    fn commit_stage(&mut self, stage_id: u32) -> ProverStatus {
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        debug!("{}: ··· Computing commit stage {}", Self::MY_NAME, stage_id);

        timer_start!(STARK_COMMIT_STAGE_, stage_id);

        let p_proof = self.p_proof.unwrap();
        let p_steps = self.p_steps;
        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };

        can_stage_be_calculated_c(p_steps, stage_id as u64);

        commit_stage_c(p_stark, element_type, stage_id as u64, p_steps, p_proof);

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
            self.compute_fri_folding(opening_id, proof_ctx, transcript);
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

    fn get_map_offsets(&self, stage: &str, is_extended: bool) -> u64 {
        get_map_offsets_c(self.p_starkinfo, stage, is_extended)
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
            println!("Root is {:?}", root);

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
            proof_ctx.challenges.as_mut().unwrap().extend(std::iter::repeat(F::zero()).take(4));
            let challenges: &Vec<F> = proof_ctx.challenges.as_ref().unwrap();
            transcript.get_challenge(&challenges[challenges.len() - 4] as *const F as *mut c_void);
        }
    }

    fn get_proof(&self) -> *mut c_void {
        self.p_proof.unwrap()
    }
}

impl<F: Field> StarkProver<F> {
    // Return the total number of elements needed to compute the STARK
    pub fn get_total_bytes(&self) -> usize {
        get_map_totaln_c(self.p_starkinfo) as usize * std::mem::size_of::<F>()
    }

    fn compute_evals(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<F>) {
        let p_stark = self.p_stark;
        let p_steps = self.p_steps;
        let p_proof = self.p_proof.unwrap();

        debug!("{}: ··· Computing evaluations", Self::MY_NAME);

        compute_evals_c(p_stark, p_steps, p_proof);
    }

    fn compute_fri_pol(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<F>) {
        let p_stark = self.p_stark;
        let p_steps = self.p_steps;

        debug!("{}: ··· Computing FRI Polynomial", Self::MY_NAME);

        compute_fri_pol_c(p_stark, self.stark_info.n_stages as u64 + 2, p_steps);
    }

    fn compute_fri_folding(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript) {
        let p_stark = self.p_stark;
        let p_proof = self.p_proof.unwrap();
        let p_steps = self.p_steps;
        let step = opening_id - 3;

        let steps = &self.stark_info.stark_struct.steps;
        let n_steps = steps.len();

        if step < (n_steps - 1) as u32 {
            debug!(
                "{}: ··· Computing FRI folding from {} to {}",
                Self::MY_NAME,
                steps[step as usize].n_bits,
                steps[(step + 1) as usize].n_bits
            );
        }
        let challenges: &Vec<F> = proof_ctx.challenges.as_ref().unwrap();
        let challenge: Vec<F> = challenges.iter().skip(challenges.len() - 4).cloned().collect();

        compute_fri_folding_c(p_stark, step as u64, p_steps, challenge.as_ptr() as *mut c_void, p_proof);

        if step < (n_steps - 1) as u32 {
            let root = fri_proof_get_tree_root_c(p_proof, (step + 1) as u64, 0);
            transcript.add_elements(root, self.n_field_elements);
        } else {
            let hash: Vec<F> = vec![F::zero(); self.n_field_elements];
            let n_hash = (1 << (steps[n_steps - 1].n_bits)) * Self::FIELD_EXTENSION as u64;
            let fri_pol = get_fri_pol_c(p_stark, p_steps);
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
        let challenge: Vec<F> = challenges.iter().skip(challenges.len() - 4).cloned().collect();

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
