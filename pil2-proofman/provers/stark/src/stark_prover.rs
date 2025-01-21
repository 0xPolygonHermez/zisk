use core::panic;
use std::fs::File;
use std::io::Read;

use std::any::type_name;
use std::sync::Arc;

use proofman_common::{
    ConstraintInfo, ProofCtx, ProofType, Prover, ProverInfo, ProverStatus, StepsParams, SetupCtx, StarkInfo,
};
use log::{debug, trace};
use transcript::FFITranscript;
use proofman_starks_lib_c::*;
use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;
use p3_field::Field;
use p3_field::PrimeField64;

use std::os::raw::c_void;
use std::marker::PhantomData;

#[allow(dead_code)]
pub struct StarkProver<F: Field> {
    global_idx: usize,
    air_id: usize,
    airgroup_id: usize,
    air_instance_id: usize,
    pub p_stark: *mut c_void,
    p_stark_info: *mut c_void,
    stark_info: StarkInfo,
    n_field_elements: usize,
    merkle_tree_arity: u64,
    merkle_tree_custom: bool,
    p_proof: *mut c_void,
    constraints_to_check: Vec<usize>,
    _marker: PhantomData<F>, // Add PhantomData to track the type F
}

impl<F: Field> StarkProver<F> {
    const MY_NAME: &'static str = "estrkPrv";

    const HASH_SIZE: usize = 4;
    const FIELD_EXTENSION: usize = 3;

    pub fn new(
        sctx: Arc<SetupCtx>,
        airgroup_id: usize,
        air_id: usize,
        air_instance_id: usize,
        global_idx: usize,
        constraints_to_check: Vec<usize>,
    ) -> Self {
        let setup = sctx.get_setup(airgroup_id, air_id);

        let p_stark = starks_new_c((&setup.p_setup).into(), setup.get_const_tree_ptr());

        let stark_info = setup.stark_info.clone();

        let (n_field_elements, merkle_tree_arity, merkle_tree_custom) =
            if stark_info.stark_struct.verification_hash_type == "BN128" {
                (1, stark_info.stark_struct.merkle_tree_arity, stark_info.stark_struct.merkle_tree_custom)
            } else {
                (Self::HASH_SIZE, 2, false)
            };

        let p_stark_info = setup.p_setup.p_stark_info;

        let p_proof = fri_proof_new_c((&setup.p_setup).into(), air_instance_id as u64);

        Self {
            global_idx,
            air_id,
            airgroup_id,
            air_instance_id,
            p_stark_info,
            p_stark,
            p_proof,
            stark_info,
            n_field_elements,
            merkle_tree_arity,
            merkle_tree_custom,
            constraints_to_check,
            _marker: PhantomData,
        }
    }
}

impl<F: Field> Prover<F> for StarkProver<F> {
    fn build(&mut self, pctx: Arc<ProofCtx<F>>) {
        let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = air_instances.get_mut(&self.global_idx).unwrap();
        air_instance.init_aux_trace(get_map_totaln_c(self.p_stark_info, false) as usize);
        air_instance.init_evals(self.stark_info.ev_map.len() * Self::FIELD_EXTENSION);

        let n_custom_commits = self.stark_info.custom_commits.len();

        for commit_id in 0..n_custom_commits {
            let n_cols = *self
                .stark_info
                .map_sections_n
                .get(&(self.stark_info.custom_commits[commit_id].name.clone() + "0"))
                .unwrap() as usize;

            if air_instance.custom_commits[commit_id].is_empty() {
                air_instance.init_custom_commit(commit_id, (1 << self.stark_info.stark_struct.n_bits) * n_cols);
            }

            let extended_size = (1 << self.stark_info.stark_struct.n_bits_ext) * n_cols;
            let mt_nodes = (2 * (1 << self.stark_info.stark_struct.n_bits_ext) - 1) * self.n_field_elements;
            air_instance.init_custom_commit_extended(commit_id, extended_size + mt_nodes);
        }

        let n_airgroup_values = self.stark_info.airgroupvalues_map.as_ref().unwrap().len();
        let n_air_values = self.stark_info.airvalues_map.as_ref().unwrap().len();

        if n_air_values > 0 && air_instance.airvalues.is_empty() {
            air_instance.init_airvalues(n_air_values * Self::FIELD_EXTENSION);
        }

        if n_airgroup_values > 0 && air_instance.airgroup_values.is_empty() {
            air_instance.init_airgroup_values(n_airgroup_values * Self::FIELD_EXTENSION);
        }

        air_instance.set_prover_initialized();
    }

    fn free(&mut self) {
        starks_free_c(self.p_stark);
        fri_proof_free_c(self.p_proof);
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

    fn verify_constraints(&self, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) -> Vec<ConstraintInfo> {
        let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = air_instances.get_mut(&self.global_idx).unwrap();

        let setup = sctx.get_setup(self.airgroup_id, self.air_id);

        let steps_params = StepsParams {
            trace: air_instance.get_trace_ptr(),
            aux_trace: air_instance.get_aux_trace_ptr(),
            public_inputs: pctx.get_publics_ptr(),
            proof_values: pctx.get_proof_values_ptr(),
            challenges: pctx.get_challenges_ptr(),
            airgroup_values: air_instance.get_airgroup_values_ptr(),
            airvalues: air_instance.get_airvalues_ptr(),
            evals: air_instance.get_evals_ptr(),
            xdivxsub: std::ptr::null_mut(),
            p_const_pols: setup.get_const_ptr(),
            p_const_tree: std::ptr::null_mut(),
            custom_commits: air_instance.get_custom_commits_ptr(),
            custom_commits_extended: [std::ptr::null_mut(); 10],
        };

        let p_setup = (&setup.p_setup).into();

        let n_constraints = get_n_constraints_c(p_setup);

        let mut constraints_info = vec![ConstraintInfo::default(); n_constraints as usize];

        if !self.constraints_to_check.is_empty() {
            constraints_info.iter_mut().for_each(|constraint| constraint.skip = true);
            for constraint_id in &self.constraints_to_check {
                constraints_info[*constraint_id].skip = false;
            }
        }

        verify_constraints_c(p_setup, (&steps_params).into(), constraints_info.as_mut_ptr() as *mut c_void);

        constraints_info
    }

    fn calculate_stage(&mut self, stage_id: u32, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = air_instances.get_mut(&self.global_idx).unwrap();

        let setup = sctx.get_setup(self.airgroup_id, self.air_id);

        let steps_params = StepsParams {
            trace: air_instance.get_trace_ptr(),
            aux_trace: air_instance.get_aux_trace_ptr(),
            public_inputs: pctx.get_publics_ptr(),
            proof_values: pctx.get_proof_values_ptr(),
            challenges: pctx.get_challenges_ptr(),
            airgroup_values: air_instance.get_airgroup_values_ptr(),
            airvalues: air_instance.get_airvalues_ptr(),
            evals: air_instance.get_evals_ptr(),
            xdivxsub: std::ptr::null_mut(),
            p_const_pols: setup.get_const_ptr(),
            p_const_tree: setup.get_const_tree_ptr(),
            custom_commits: air_instance.get_custom_commits_ptr(),
            custom_commits_extended: air_instance.get_custom_commits_extended_ptr(),
        };

        if stage_id as usize <= pctx.global_info.n_challenges.len() {
            if self
                .stark_info
                .cm_pols_map
                .as_ref()
                .expect("REASON")
                .iter()
                .any(|cm_pol| cm_pol.stage == stage_id as u64 && cm_pol.im_pol)
            {
                let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;
                debug!(
                    "{}: ··· Computing intermediate polynomials of instance {} of {}",
                    Self::MY_NAME,
                    self.air_instance_id,
                    air_name
                );

                calculate_impols_expressions_c(self.p_stark, stage_id as u64, (&steps_params).into());
            }

            if stage_id as usize == pctx.global_info.n_challenges.len() {
                let p_proof = self.p_proof;
                fri_proof_set_airgroup_values_c(p_proof, steps_params.airgroup_values);
                fri_proof_set_air_values_c(p_proof, steps_params.airvalues);
            }
        } else {
            let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;
            debug!(
                "{}: ··· Computing Quotient Polynomial of instance {} of {}",
                Self::MY_NAME,
                self.air_instance_id,
                air_name
            );
            calculate_quotient_polynomial_c(self.p_stark, (&steps_params).into());
        }
    }

    fn commit_stage(&mut self, stage_id: u32, pctx: Arc<ProofCtx<F>>) -> ProverStatus {
        let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = air_instances.get_mut(&self.global_idx).unwrap();

        let p_stark = self.p_stark;
        let p_proof = self.p_proof;

        let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;
        debug!(
            "{}: ··· Committing prover {}: instance {} of {}",
            Self::MY_NAME,
            self.global_idx,
            self.air_instance_id,
            air_name
        );

        let witness = match stage_id == 1 {
            true => air_instance.get_trace_ptr(),
            false => std::ptr::null_mut(),
        };

        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };

        commit_stage_c(
            p_stark,
            element_type,
            stage_id as u64,
            witness,
            air_instance.get_aux_trace_ptr(),
            p_proof,
            pctx.get_buff_helper_ptr(),
        );

        if stage_id <= self.num_stages() + 1 {
            ProverStatus::CommitStage
        } else {
            ProverStatus::OpeningStage
        }
    }

    fn opening_stage(&mut self, opening_id: u32, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) -> ProverStatus {
        let steps_fri: Vec<usize> = pctx.global_info.steps_fri.iter().map(|step| step.n_bits).collect();
        let last_stage_id = steps_fri.len() as u32 + 3;
        if opening_id == 1 {
            self.compute_evals(opening_id, sctx, pctx);
        } else if opening_id == 2 {
            self.compute_fri_pol(opening_id, sctx, pctx);
        } else if opening_id < last_stage_id {
            let global_step_fri = steps_fri[(opening_id - 3) as usize];
            let step_index =
                self.stark_info.stark_struct.steps.iter().position(|s| s.n_bits as usize == global_step_fri);
            if let Some(step_index) = step_index {
                self.compute_fri_folding(step_index as u32, pctx);
            } else {
                let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;
                debug!(
                    "{}: ··· Skipping FRI folding of instance {} of {}",
                    Self::MY_NAME,
                    self.air_instance_id,
                    air_name
                );
            }
        } else if opening_id == last_stage_id {
            self.compute_fri_queries(opening_id, pctx);
        } else {
            panic!("Opening stage not implemented");
        }

        if opening_id == last_stage_id {
            ProverStatus::StagesCompleted
        } else {
            ProverStatus::OpeningStage
        }
    }

    fn commit_custom_commits_stage(&mut self, stage_id: u32, pctx: Arc<ProofCtx<F>>) -> Vec<u64> {
        let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = air_instances.get_mut(&self.global_idx).unwrap();

        let p_stark = self.p_stark;
        let p_proof = self.p_proof;

        let n_custom_commits = self.stark_info.custom_commits.len();

        if n_custom_commits == 0 {
            return Vec::new();
        }

        let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;
        debug!(
            "{}: ··· Committing custom commits for prover {}: instance {} of {}",
            Self::MY_NAME,
            self.global_idx,
            self.air_instance_id,
            air_name
        );

        let mut custom_publics = Vec::new();
        for commit_id in 0..n_custom_commits {
            let custom_commits_stage = self.stark_info.custom_commits_map[commit_id]
                .as_ref()
                .expect("REASON")
                .iter()
                .any(|custom_commit| custom_commit.stage == stage_id as u64);

            if custom_commits_stage {
                extend_and_merkelize_custom_commit_c(
                    p_stark,
                    commit_id as u64,
                    stage_id as u64,
                    air_instance.custom_commits[commit_id].as_ptr() as *mut u8,
                    air_instance.custom_commits_extended[commit_id].as_ptr() as *mut u8,
                    p_proof,
                    pctx.get_buff_helper_ptr(),
                    "",
                );
            }

            let mut value = vec![Goldilocks::zero(); self.n_field_elements];
            treesGL_get_root_c(
                p_stark,
                (self.stark_info.n_stages + 2 + commit_id as u32) as u64,
                value.as_mut_ptr() as *mut u8,
            );
            if !self.stark_info.custom_commits[commit_id].public_values.is_empty() {
                assert!(
                    self.n_field_elements == self.stark_info.custom_commits[commit_id].public_values.len(),
                    "Invalid public values size"
                );
                for (idx, val) in value.iter().enumerate() {
                    custom_publics.push(self.stark_info.custom_commits[commit_id].public_values[idx].idx);
                    custom_publics.push(val.as_canonical_u64());
                }
            }
        }
        custom_publics
    }

    fn calculate_xdivxsub(&mut self, pctx: Arc<ProofCtx<F>>) {
        let challenges_guard = pctx.challenges.values.read().unwrap();

        let challenges_map = self.stark_info.challenges_map.as_ref().unwrap();

        let mut xi_challenge_index: usize = 0;
        for (i, challenge) in challenges_map.iter().enumerate() {
            if challenge.stage == (Self::num_stages(self) + 2) as u64 && challenge.stage_id == 0_u64 {
                xi_challenge_index = i;
                break;
            }
        }

        let xi_challenge = &(*challenges_guard)[xi_challenge_index * Self::FIELD_EXTENSION] as *const F as *mut c_void;
        calculate_xdivxsub_c(self.p_stark, xi_challenge, pctx.get_buff_helper_ptr());
    }

    fn calculate_lev(&mut self, pctx: Arc<ProofCtx<F>>) {
        let challenges_guard = pctx.challenges.values.read().unwrap();

        let challenges_map = self.stark_info.challenges_map.as_ref().unwrap();

        let mut xi_challenge_index: usize = 0;
        for (i, challenge) in challenges_map.iter().enumerate() {
            if challenge.stage == (Self::num_stages(self) + 2) as u64 && challenge.stage_id == 0_u64 {
                xi_challenge_index = i;
                break;
            }
        }

        let xi_challenge = &(*challenges_guard)[xi_challenge_index * Self::FIELD_EXTENSION] as *const F as *mut c_void;
        compute_lev_c(self.p_stark, xi_challenge, pctx.get_buff_helper_ptr());
    }

    fn get_buff_helper_size(&self, _proof_ctx: Arc<ProofCtx<F>>) -> usize {
        // if pctx.options.verify_constraints {

        // } else {
        self.stark_info.get_buff_helper_size()
        // }
    }

    fn calculate_hash(&self, values: Vec<F>) -> Vec<F> {
        let hash = vec![F::zero(); self.n_field_elements];
        calculate_hash_c(self.p_stark, hash.as_ptr() as *mut u8, values.as_ptr() as *mut u8, values.len() as u64);
        hash
    }

    fn get_transcript_values(&self, stage: u64, pctx: Arc<ProofCtx<F>>) -> Vec<F> {
        let values = self.get_transcript_values_u64(stage, pctx).iter().map(|v| F::from_canonical_u64(*v)).collect();
        values
    }

    fn get_transcript_values_u64(&self, stage: u64, pctx: Arc<ProofCtx<F>>) -> Vec<u64> {
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;

        let mut value = vec![Goldilocks::zero(); self.n_field_elements];
        if stage <= (Self::num_stages(self) + 1) as u64 {
            let n_airvals_stage: usize = self
                .stark_info
                .airvalues_map
                .as_ref()
                .map(|map| map.iter().filter(|entry| entry.stage == stage).count())
                .unwrap_or(0);

            if stage == 1 || n_airvals_stage > 0 {
                let size = if stage == 1 {
                    2 * self.n_field_elements + n_airvals_stage
                } else {
                    self.n_field_elements + n_airvals_stage * Self::FIELD_EXTENSION
                };
                let mut values_hash = vec![F::zero(); size];

                let verkey = pctx
                    .global_info
                    .get_air_setup_path(self.airgroup_id, self.air_id, &ProofType::Basic)
                    .display()
                    .to_string()
                    + ".verkey.json";

                let mut file = File::open(&verkey).expect("Unable to open file");
                let mut json_str = String::new();
                file.read_to_string(&mut json_str).expect("Unable to read file");
                let vk: Vec<u64> = serde_json::from_str(&json_str).expect("REASON");
                for j in 0..self.n_field_elements {
                    values_hash[j] = F::from_canonical_u64(vk[j]);
                }

                let mut root = vec![F::zero(); self.n_field_elements];
                treesGL_get_root_c(p_stark, stage - 1, root.as_mut_ptr() as *mut u8);
                trace!(
                    "{}: ··· MerkleTree root for stage {} of instance {} of {} is: {:?}",
                    Self::MY_NAME,
                    stage,
                    self.air_instance_id,
                    air_name,
                    root,
                );
                for (j, &root_value) in root.iter().enumerate().take(self.n_field_elements) {
                    let index = if stage == 1 { self.n_field_elements + j } else { j };
                    values_hash[index] = root_value;
                }
                let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
                let air_instance = air_instances.get_mut(&self.global_idx).unwrap();
                let airvalues_map = self.stark_info.airvalues_map.as_ref().unwrap();
                let mut p = 0;
                let mut count = 0;
                for air_value in airvalues_map {
                    if air_value.stage > stage {
                        break;
                    }
                    if air_value.stage == 1 {
                        if stage == 1 {
                            values_hash[2 * self.n_field_elements + count] = air_instance.airvalues[p];
                            count += 1;
                        }
                        p += 1;
                    } else {
                        if air_value.stage == stage {
                            values_hash[self.n_field_elements + count] = air_instance.airvalues[p];
                            values_hash[self.n_field_elements + count + 1] = air_instance.airvalues[p + 1];
                            values_hash[self.n_field_elements + count + 2] = air_instance.airvalues[p + 2];
                            count += 3;
                        }
                        p += 3;
                    }
                }

                calculate_hash_c(
                    p_stark,
                    value.as_mut_ptr() as *mut u8,
                    values_hash.as_mut_ptr() as *mut u8,
                    size as u64,
                );
            } else {
                treesGL_get_root_c(p_stark, stage - 1, value.as_mut_ptr() as *mut u8);
            }
        } else if stage == (Self::num_stages(self) + 2) as u64 {
            let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = air_instances.get_mut(&self.global_idx).unwrap();
            calculate_hash_c(
                p_stark,
                value.as_mut_ptr() as *mut u8,
                air_instance.get_evals_ptr(),
                (self.stark_info.ev_map.len() * Self::FIELD_EXTENSION) as u64,
            );
        } else if stage > (Self::num_stages(self) + 3) as u64 {
            let steps = &self.stark_info.stark_struct.steps;

            let steps_fri: Vec<usize> = pctx.global_info.steps_fri.iter().map(|step| step.n_bits).collect();
            let step_index =
                self.stark_info.stark_struct.steps.iter().position(|s| {
                    s.n_bits as usize == steps_fri[(stage as u32 - (Self::num_stages(self) + 4)) as usize]
                });

            if let Some(step_index) = step_index {
                let n_steps = steps.len() - 1;
                if step_index < n_steps {
                    let p_proof = self.p_proof;
                    fri_proof_get_tree_root_c(p_proof, value.as_mut_ptr() as *mut u8, step_index as u64);
                } else {
                    let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
                    let air_instance = air_instances.get_mut(&self.global_idx).unwrap();
                    let n_hash = (1 << (steps[n_steps].n_bits)) * Self::FIELD_EXTENSION as u64;
                    let fri_pol = get_fri_pol_c(self.p_stark_info, air_instance.get_aux_trace_ptr());
                    calculate_hash_c(p_stark, value.as_mut_ptr() as *mut u8, fri_pol as *mut u8, n_hash);
                }
            }
        }
        let mut value64: Vec<u64> = Vec::new();
        for v in value {
            value64.push(v.as_canonical_u64());
        }
        value64
    }

    fn get_challenges(&self, stage_id: u32, pctx: Arc<ProofCtx<F>>, transcript: &FFITranscript) {
        if stage_id == 1 {
            return;
        }

        let mpi_rank = pctx.dctx_get_rank();

        if stage_id <= self.num_stages() + 3 {
            //num stages + 1 + evals + fri_pol (then starts fri folding...)

            let challenges_map = self.stark_info.challenges_map.as_ref().unwrap();

            let challenges = &*pctx.challenges.values.read().unwrap();
            for i in 0..challenges_map.len() {
                if challenges_map[i].stage == stage_id as u64 {
                    let challenge = &challenges[i * Self::FIELD_EXTENSION];
                    transcript.get_challenge(challenge as *const F as *mut c_void);
                    if mpi_rank == 0 {
                        debug!(
                            "{}: ··· Global challenge: [{}, {}, {}]",
                            Self::MY_NAME,
                            challenges[i * Self::FIELD_EXTENSION],
                            challenges[i * Self::FIELD_EXTENSION + 1],
                            challenges[i * Self::FIELD_EXTENSION + 2],
                        );
                    }
                }
            }
        } else {
            //Fri folding + . queries: add one challenge for each step
            let mut challenges_guard = pctx.challenges.values.write().unwrap();

            challenges_guard.extend(std::iter::repeat(F::zero()).take(3));
            transcript.get_challenge(&(*challenges_guard)[challenges_guard.len() - 3] as *const F as *mut c_void);
            if mpi_rank == 0 {
                debug!(
                    "{}: ··· Global challenge: [{}, {}, {}]",
                    Self::MY_NAME,
                    challenges_guard[challenges_guard.len() - 3],
                    challenges_guard[challenges_guard.len() - 2],
                    challenges_guard[challenges_guard.len() - 1],
                );
            }
        }
    }

    fn get_proof(&self) -> *mut c_void {
        self.p_proof
    }

    fn get_stark(&self) -> *mut c_void {
        self.p_stark
    }

    fn get_zkin_proof(&self, pctx: Arc<ProofCtx<F>>, output_dir: &str) -> *mut c_void {
        let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
        let global_info_file: &str = global_info_path.to_str().unwrap();

        let proof_dir = match &pctx.options.debug_info.save_proofs_to_file {
            true => output_dir,
            false => "",
        };

        fri_proof_get_zkinproof_c(
            self.p_proof,
            pctx.get_publics_ptr(),
            pctx.get_challenges_ptr(),
            pctx.get_proof_values_ptr(),
            global_info_file,
            proof_dir,
        )
    }

    fn get_prover_info(&self) -> ProverInfo {
        ProverInfo { airgroup_id: self.airgroup_id, air_id: self.air_id, air_instance_id: self.air_instance_id }
    }

    fn get_proof_challenges(&self, global_steps: Vec<usize>, global_challenges: Vec<F>) -> Vec<F> {
        let mut challenges: Vec<F> = Vec::new();

        let n_challenges_stages = self.stark_info.challenges_map.as_ref().unwrap().len();
        for ch in 0..n_challenges_stages {
            challenges.push(global_challenges[ch * 3]);
            challenges.push(global_challenges[ch * 3 + 1]);
            challenges.push(global_challenges[ch * 3 + 2]);
        }

        for s in self.stark_info.stark_struct.steps.clone().into_iter() {
            let step_index = global_steps.iter().position(|step| *step == s.n_bits as usize).expect("REASON");
            challenges.push(global_challenges[(n_challenges_stages + step_index) * Self::FIELD_EXTENSION]);
            challenges.push(global_challenges[(n_challenges_stages + step_index) * Self::FIELD_EXTENSION + 1]);
            challenges.push(global_challenges[(n_challenges_stages + step_index) * Self::FIELD_EXTENSION + 2]);
        }

        challenges.push(global_challenges[(n_challenges_stages + global_steps.len()) * Self::FIELD_EXTENSION]);
        challenges.push(global_challenges[(n_challenges_stages + global_steps.len()) * Self::FIELD_EXTENSION + 1]);
        challenges.push(global_challenges[(n_challenges_stages + global_steps.len()) * Self::FIELD_EXTENSION + 2]);

        challenges
    }
}

impl<F: Field> StarkProver<F> {
    fn compute_evals(&mut self, _opening_id: u32, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;
        debug!("{}: ··· Calculating evals of instance {} of {}", Self::MY_NAME, self.air_instance_id, air_name);
        let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = air_instances.get_mut(&self.global_idx).unwrap();

        let setup = sctx.get_setup(self.airgroup_id, self.air_id);

        let p_stark = self.p_stark;
        let p_proof = self.p_proof;

        let steps_params = StepsParams {
            trace: std::ptr::null_mut(),
            aux_trace: air_instance.get_aux_trace_ptr(),
            public_inputs: std::ptr::null_mut(),
            proof_values: std::ptr::null_mut(),
            challenges: std::ptr::null_mut(),
            airgroup_values: std::ptr::null_mut(),
            airvalues: std::ptr::null_mut(),
            evals: air_instance.get_evals_ptr(),
            xdivxsub: std::ptr::null_mut(),
            p_const_pols: std::ptr::null_mut(),
            p_const_tree: setup.get_const_tree_ptr(),
            custom_commits: air_instance.get_custom_commits_ptr(),
            custom_commits_extended: air_instance.get_custom_commits_extended_ptr(),
        };

        compute_evals_c(p_stark, (&steps_params).into(), pctx.get_buff_helper_ptr(), p_proof);
    }

    fn compute_fri_pol(&mut self, _opening_id: u32, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;
        debug!(
            "{}: ··· Calculating FRI polynomial of instance {} of {}",
            Self::MY_NAME,
            self.air_instance_id,
            air_name
        );
        let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = air_instances.get_mut(&self.global_idx).unwrap();

        let setup = sctx.get_setup(self.airgroup_id, self.air_id);

        let p_stark = self.p_stark;

        let steps_params = StepsParams {
            trace: std::ptr::null_mut(),
            aux_trace: air_instance.get_aux_trace_ptr(),
            public_inputs: pctx.get_publics_ptr(),
            proof_values: pctx.get_proof_values_ptr(),
            challenges: pctx.get_challenges_ptr(),
            airgroup_values: air_instance.get_airgroup_values_ptr(),
            airvalues: air_instance.get_airvalues_ptr(),
            evals: air_instance.get_evals_ptr(),
            xdivxsub: pctx.get_buff_helper_ptr(),
            p_const_pols: setup.get_const_ptr(),
            p_const_tree: setup.get_const_tree_ptr(),
            custom_commits: air_instance.get_custom_commits_ptr(),
            custom_commits_extended: air_instance.get_custom_commits_extended_ptr(),
        };

        calculate_fri_polynomial_c(p_stark, (&steps_params).into());
    }

    fn compute_fri_folding(&mut self, step_index: u32, pctx: Arc<ProofCtx<F>>) {
        let p_proof = self.p_proof;

        let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;

        let steps = &self.stark_info.stark_struct.steps;
        let n_steps = (steps.len() - 1) as u32;
        if step_index == n_steps {
            debug!(
                "{}: ··· Calculating final FRI polynomial of instance {} of {}",
                Self::MY_NAME,
                self.air_instance_id,
                air_name
            );
        } else {
            debug!(
                "{}: ··· Calculating FRI folding of instance {} of {}",
                Self::MY_NAME,
                self.air_instance_id,
                air_name
            );
        }

        let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
        let air_instance = air_instances.get_mut(&self.global_idx).unwrap();

        let fri_pol = get_fri_pol_c(self.p_stark_info, air_instance.get_aux_trace_ptr());

        let challenges_guard = pctx.challenges.values.read().unwrap();
        let challenge: Vec<F> = challenges_guard.iter().skip(challenges_guard.len() - 3).cloned().collect();

        let current_bits = steps[step_index as usize].n_bits;
        let prev_bits = if step_index == 0 { current_bits } else { steps[(step_index - 1) as usize].n_bits };

        compute_fri_folding_c(
            step_index as u64,
            fri_pol as *mut u8,
            challenge.as_ptr() as *mut u8,
            self.stark_info.stark_struct.n_bits_ext,
            prev_bits,
            current_bits,
        );

        if step_index != n_steps {
            let next_bits = steps[(step_index + 1) as usize].n_bits;
            compute_fri_merkelize_c(
                self.p_stark,
                p_proof,
                step_index as u64,
                fri_pol as *mut u8,
                current_bits,
                next_bits,
            );
        }
    }

    fn compute_fri_queries(&mut self, _opening_id: u32, pctx: Arc<ProofCtx<F>>) {
        let p_stark = self.p_stark;
        let p_proof = self.p_proof;

        let n_queries = self.stark_info.stark_struct.n_queries;
        let steps = &self.stark_info.stark_struct.steps;
        let air_name = &pctx.global_info.airs[self.airgroup_id][self.air_id].name;
        debug!("{}: ··· Calculating FRI queries of instance {} of {}", Self::MY_NAME, self.air_instance_id, air_name);

        let mut fri_queries = vec![u64::default(); n_queries as usize];

        let challenges_guard = pctx.challenges.values.read().unwrap();

        let challenge: Vec<F> = challenges_guard.iter().skip(challenges_guard.len() - 3).cloned().collect();

        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };
        let transcript_permutation =
            FFITranscript::new(p_stark, element_type, self.merkle_tree_arity, self.merkle_tree_custom);

        transcript_permutation.add_elements(challenge.as_ptr() as *mut u8, Self::FIELD_EXTENSION);
        transcript_permutation.get_permutations(
            fri_queries.as_mut_ptr(),
            n_queries,
            self.stark_info.stark_struct.steps[0].n_bits,
        );

        trace!(
            "{}: ··· FRI queries of instance {} of {} are: {:?}",
            Self::MY_NAME,
            self.air_instance_id,
            air_name,
            &fri_queries,
        );

        let air_instances = pctx.air_instance_repo.air_instances.read().unwrap();
        let air_instance = air_instances.get(&self.global_idx).unwrap();
        let fri_pol = get_fri_pol_c(self.p_stark_info, air_instance.get_aux_trace_ptr());

        let n_trees = self.num_stages() + 2 + self.stark_info.custom_commits.len() as u32;
        compute_queries_c(p_stark, p_proof, fri_queries.as_mut_ptr(), n_queries, n_trees as u64);
        for (step, _) in steps.iter().enumerate().take(self.stark_info.stark_struct.steps.len()).skip(1) {
            compute_fri_queries_c(
                self.p_stark,
                p_proof,
                fri_queries.as_mut_ptr(),
                n_queries,
                step as u64,
                steps[step].n_bits,
            );
        }

        set_fri_final_pol_c(
            p_proof,
            fri_pol as *mut u8,
            self.stark_info.stark_struct.steps[self.stark_info.stark_struct.steps.len() - 1].n_bits,
        );
    }
}

unsafe impl<F: Field> Send for StarkProver<F> {}
unsafe impl<F: Field> Sync for StarkProver<F> {}
