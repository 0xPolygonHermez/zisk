use core::panic;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use std::any::type_name;
use std::sync::Arc;

use proofman_common::{
    BufferAllocator, ConstraintInfo, ConstraintsResults, ProofCtx, ProofType, Prover, ProverInfo, ProverStatus,
    StepsParams, SetupCtx, StarkInfo,
};
use log::{debug, trace};
use transcript::FFITranscript;
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use proofman_starks_lib_c::*;
use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;
use p3_field::Field;
use p3_field::PrimeField64;

use std::os::raw::c_void;
use std::marker::PhantomData;

#[repr(C)]
pub struct VecU64Result {
    pub n_elements: u64,
    pub ids: *mut u64,
}

#[allow(dead_code)]
pub struct StarkProver<F: Field> {
    initialized: bool,
    prover_idx: usize,
    air_id: usize,
    airgroup_id: usize,
    instance_id: usize,
    pub p_stark: *mut c_void,
    p_stark_info: *mut c_void,
    stark_info: StarkInfo,
    n_field_elements: usize,
    merkle_tree_arity: u64,
    merkle_tree_custom: bool,
    p_proof: *mut c_void,
    _marker: PhantomData<F>, // Add PhantomData to track the type F
}

impl<F: Field> StarkProver<F> {
    const MY_NAME: &'static str = "estrkPrv";

    const HASH_SIZE: usize = 4;
    const FIELD_EXTENSION: usize = 3;

    pub fn new(sctx: Arc<SetupCtx>, airgroup_id: usize, air_id: usize, instance_id: usize, prover_idx: usize) -> Self {
        let setup = sctx.get_setup(airgroup_id, air_id);

        let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

        let p_stark = starks_new_c((&setup.p_setup).into(), const_tree_ptr);

        let stark_info = setup.stark_info.clone();

        let (n_field_elements, merkle_tree_arity, merkle_tree_custom) =
            if stark_info.stark_struct.verification_hash_type == "BN128" {
                (1, stark_info.stark_struct.merkle_tree_arity, stark_info.stark_struct.merkle_tree_custom)
            } else {
                (Self::HASH_SIZE, 2, false)
            };

        let p_stark_info = setup.p_setup.p_stark_info;

        let p_proof = fri_proof_new_c((&setup.p_setup).into());

        Self {
            initialized: true,
            prover_idx,
            air_id,
            airgroup_id,
            instance_id,
            p_stark_info,
            p_stark,
            p_proof,
            stark_info,
            n_field_elements,
            merkle_tree_arity,
            merkle_tree_custom,
            _marker: PhantomData,
        }
    }
}

impl<F: Field> Prover<F> for StarkProver<F> {
    fn build(&mut self, proof_ctx: Arc<ProofCtx<F>>) {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        //initialize the common challenges if have not been initialized by another prover
        let challenges =
            vec![F::zero(); self.stark_info.challenges_map.as_ref().unwrap().len() * Self::FIELD_EXTENSION];
        *proof_ctx.challenges.challenges.write().unwrap() = challenges;

        let number_stage1_commits = *self.stark_info.map_sections_n.get("cm1").unwrap() as usize;
        for i in 0..number_stage1_commits {
            air_instance.set_commit_calculated(i);
        }

        self.initialized = true;
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

    fn verify_constraints(&self, setup_ctx: Arc<SetupCtx>, proof_ctx: Arc<ProofCtx<F>>) -> Vec<ConstraintInfo> {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let setup = setup_ctx.get_setup(self.airgroup_id, self.air_id);

        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
        let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

        let steps_params = StepsParams {
            buffer: air_instance.get_buffer_ptr() as *mut c_void,
            public_inputs: (*public_inputs_guard).as_ptr() as *mut c_void,
            challenges: (*challenges_guard).as_ptr() as *mut c_void,
            airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
            airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
            evals: air_instance.evals.as_ptr() as *mut c_void,
            xdivxsub: std::ptr::null_mut(),
            p_const_pols: const_pols_ptr,
            p_const_tree: const_tree_ptr,
            custom_commits: air_instance.get_custom_commits_ptr(),
        };

        let raw_ptr = verify_constraints_c((&setup.p_setup).into(), (&steps_params).into());

        unsafe {
            let constraints_result = Box::from_raw(raw_ptr as *mut ConstraintsResults);
            std::slice::from_raw_parts(constraints_result.constraints_info, constraints_result.n_constraints as usize)
        }
        .to_vec()
    }

    fn calculate_stage(&mut self, stage_id: u32, setup_ctx: Arc<SetupCtx>, proof_ctx: Arc<ProofCtx<F>>) {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let n_commits = self.stark_info.cm_pols_map.as_ref().expect("REASON").len();

        let setup = setup_ctx.get_setup(self.airgroup_id, self.air_id);

        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
        let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

        let steps_params = StepsParams {
            buffer: air_instance.get_buffer_ptr() as *mut c_void,
            public_inputs: (*public_inputs_guard).as_ptr() as *mut c_void,
            challenges: (*challenges_guard).as_ptr() as *mut c_void,
            airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
            airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
            evals: air_instance.evals.as_ptr() as *mut c_void,
            xdivxsub: std::ptr::null_mut(),
            p_const_pols: const_pols_ptr,
            p_const_tree: const_tree_ptr,
            custom_commits: air_instance.get_custom_commits_ptr(),
        };

        if stage_id as usize <= proof_ctx.global_info.n_challenges.len() {
            if self
                .stark_info
                .cm_pols_map
                .as_ref()
                .expect("REASON")
                .iter()
                .any(|cm_pol| cm_pol.stage == stage_id as u64 && cm_pol.im_pol)
            {
                let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;
                debug!(
                    "{}: ··· Computing intermediate polynomials of instance {} of {}",
                    Self::MY_NAME,
                    self.instance_id,
                    air_name
                );
                for i in 0..n_commits {
                    let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                    if (cm_pol.stage < stage_id as u64 || cm_pol.stage == stage_id as u64 && !cm_pol.im_pol)
                        && !air_instance.commits_calculated.contains_key(&i)
                    {
                        panic!("Intermediate polynomials for stage {} cannot be calculated: Witness column {} is not calculated", stage_id, cm_pol.name);
                    }
                }

                calculate_impols_expressions_c(self.p_stark, stage_id as u64, (&steps_params).into());
                for i in 0..n_commits {
                    let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                    if cm_pol.stage == stage_id as u64 && cm_pol.im_pol {
                        air_instance.set_commit_calculated(i);
                    }
                }
            }

            if stage_id as usize == proof_ctx.global_info.n_challenges.len() {
                let p_proof = self.p_proof;
                fri_proof_set_airgroup_values_c(p_proof, steps_params.airgroup_values);
                fri_proof_set_air_values_c(p_proof, steps_params.airvalues);
            }
        } else {
            let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;
            debug!(
                "{}: ··· Computing Quotient Polynomial of instance {} of {}",
                Self::MY_NAME,
                self.instance_id,
                air_name
            );
            calculate_quotient_polynomial_c(self.p_stark, (&steps_params).into());
            for i in 0..n_commits {
                let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                if cm_pol.stage == (proof_ctx.global_info.n_challenges.len() + 1) as u64 {
                    air_instance.set_commit_calculated(i);
                }
            }
        }
    }

    fn check_stage(&self, stage_id: u32, proof_ctx: Arc<ProofCtx<F>>) {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let n_commits = self.stark_info.cm_pols_map.as_ref().expect("REASON").len();
        for i in 0..n_commits {
            let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
            if cm_pol.stage == stage_id as u64 && !air_instance.commits_calculated.contains_key(&i) {
                panic!("Stage {} cannot be committed: Witness column {} is not calculated", stage_id, cm_pol.name);
            }
        }

        let n_airgroupvalues = self.stark_info.airgroupvalues_map.as_ref().expect("REASON").len();
        for i in 0..n_airgroupvalues {
            let airgroup_value = self.stark_info.airgroupvalues_map.as_ref().expect("REASON").get(i).unwrap();
            if airgroup_value.stage == stage_id as u64 && !air_instance.airgroupvalue_calculated.contains_key(&i) {
                panic!(
                    "Stage {} cannot be committed: Airgroupvalue {} is not calculated",
                    stage_id, airgroup_value.name
                );
            }
        }

        let n_airvalues = self.stark_info.airvalues_map.as_ref().expect("REASON").len();
        for i in 0..n_airvalues {
            let air_value = self.stark_info.airvalues_map.as_ref().expect("REASON").get(i).unwrap();

            if air_value.stage == stage_id as u64 && !air_instance.airvalue_calculated.contains_key(&i) {
                panic!("Stage {} cannot be committed: Airvalue {} is not calculated", stage_id, air_value.name);
            }
        }

        let n_custom_commits = self.stark_info.custom_commits_map.len();
        for i in 0..n_custom_commits {
            let n_custom_commits = self.stark_info.custom_commits_map[i].as_ref().expect("REASON").len();
            for j in 0..n_custom_commits {
                let custom_pol = self.stark_info.custom_commits_map[i].as_ref().expect("REASON").get(j).unwrap();
                if stage_id as u64 == custom_pol.stage && !air_instance.custom_commits_calculated[i].contains_key(&j) {
                    panic!(
                        "Stage {} cannot be committed: Custom commit of {} that is {} is not calculated",
                        stage_id, self.stark_info.custom_commits[i].name, custom_pol.name
                    );
                }
            }
        }
    }

    fn commit_stage(&mut self, stage_id: u32, proof_ctx: Arc<ProofCtx<F>>) -> ProverStatus {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let p_stark = self.p_stark;
        let p_proof = self.p_proof;

        let buff_helper_guard = proof_ctx.buff_helper.buff_helper.read().unwrap();
        let buff_helper = (*buff_helper_guard).as_ptr() as *mut c_void;

        let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;
        if stage_id >= 1 {
            debug!(
                "{}: ··· Committing prover {}: instance {} of {}",
                Self::MY_NAME,
                self.prover_idx,
                self.instance_id,
                air_name
            );

            timer_start_trace!(STARK_COMMIT_STAGE_, stage_id);

            let buffer = air_instance.get_buffer_ptr() as *mut c_void;
            let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };

            commit_stage_c(p_stark, element_type, stage_id as u64, buffer, p_proof, buff_helper);
            timer_stop_and_log_trace!(STARK_COMMIT_STAGE_, stage_id);
        } else {
            let n_custom_commits = self.stark_info.custom_commits.len();
            for commit_id in 0..n_custom_commits {
                let custom_commits_stage = self.stark_info.custom_commits_map[commit_id]
                    .as_ref()
                    .expect("REASON")
                    .iter()
                    .any(|custom_commit| custom_commit.stage == stage_id as u64);

                if custom_commits_stage {
                    if air_instance.custom_commits[commit_id].cached_file.to_str() == Some("") {
                        extend_and_merkelize_custom_commit_c(
                            p_stark,
                            commit_id as u64,
                            stage_id as u64,
                            air_instance.custom_commits[commit_id].buffer.as_ptr() as *mut c_void,
                            p_proof,
                            buff_helper,
                            "",
                        );
                    } else {
                        load_custom_commit_c(
                            p_stark,
                            commit_id as u64,
                            stage_id as u64,
                            air_instance.custom_commits[commit_id].buffer.as_ptr() as *mut c_void,
                            p_proof,
                            air_instance.custom_commits[commit_id].cached_file.to_str().unwrap(),
                        );
                    }
                }

                let mut value = vec![Goldilocks::zero(); self.n_field_elements];
                treesGL_get_root_c(
                    p_stark,
                    (self.stark_info.n_stages + 2 + commit_id as u32) as u64,
                    value.as_mut_ptr() as *mut c_void,
                );
                if !self.stark_info.custom_commits[commit_id].public_values.is_empty() {
                    assert!(
                        self.n_field_elements == self.stark_info.custom_commits[commit_id].public_values.len(),
                        "Invalid public values size"
                    );
                    for (idx, val) in value.iter().enumerate() {
                        proof_ctx.set_public_value(
                            val.as_canonical_u64(),
                            self.stark_info.custom_commits[commit_id].public_values[idx].idx,
                        );
                    }
                }
            }
        }

        if stage_id <= self.num_stages() + 1 {
            ProverStatus::CommitStage
        } else {
            ProverStatus::OpeningStage
        }
    }

    fn opening_stage(
        &mut self,
        opening_id: u32,
        setup_ctx: Arc<SetupCtx>,
        proof_ctx: Arc<ProofCtx<F>>,
    ) -> ProverStatus {
        let steps_fri: Vec<usize> = proof_ctx.global_info.steps_fri.iter().map(|step| step.n_bits).collect();
        let last_stage_id = steps_fri.len() as u32 + 3;
        if opening_id == 1 {
            self.compute_evals(opening_id, setup_ctx, proof_ctx);
        } else if opening_id == 2 {
            self.compute_fri_pol(opening_id, setup_ctx, proof_ctx);
        } else if opening_id < last_stage_id {
            let global_step_fri = steps_fri[(opening_id - 3) as usize];
            let step_index =
                self.stark_info.stark_struct.steps.iter().position(|s| s.n_bits as usize == global_step_fri);
            if let Some(step_index) = step_index {
                self.compute_fri_folding(step_index as u32, proof_ctx);
            } else {
                let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;
                debug!("{}: ··· Skipping FRI folding of instance {} of {}", Self::MY_NAME, self.instance_id, air_name);
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

    fn calculate_xdivxsub(&mut self, proof_ctx: Arc<ProofCtx<F>>) {
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let buff_helper_guard = proof_ctx.buff_helper.buff_helper.read().unwrap();
        let xdivxsub = (*buff_helper_guard).as_ptr() as *mut c_void;

        let challenges_map = self.stark_info.challenges_map.as_ref().unwrap();

        let mut xi_challenge_index: usize = 0;
        for (i, challenge) in challenges_map.iter().enumerate() {
            if challenge.stage == (Self::num_stages(self) + 2) as u64 && challenge.stage_id == 0_u64 {
                xi_challenge_index = i;
                break;
            }
        }

        let xi_challenge = &(*challenges_guard)[xi_challenge_index * Self::FIELD_EXTENSION] as *const F as *mut c_void;
        calculate_xdivxsub_c(self.p_stark, xi_challenge, xdivxsub);
    }

    fn calculate_lev(&mut self, proof_ctx: Arc<ProofCtx<F>>) {
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let buff_helper_guard = proof_ctx.buff_helper.buff_helper.read().unwrap();
        let lev = (*buff_helper_guard).as_ptr() as *mut c_void;

        let challenges_map = self.stark_info.challenges_map.as_ref().unwrap();

        let mut xi_challenge_index: usize = 0;
        for (i, challenge) in challenges_map.iter().enumerate() {
            if challenge.stage == (Self::num_stages(self) + 2) as u64 && challenge.stage_id == 0_u64 {
                xi_challenge_index = i;
                break;
            }
        }

        let xi_challenge = &(*challenges_guard)[xi_challenge_index * Self::FIELD_EXTENSION] as *const F as *mut c_void;
        compute_lev_c(self.p_stark, xi_challenge, lev);
    }

    fn get_buff_helper_size(&self) -> usize {
        let mut max_cols = 0;
        for stage in 1..=Self::num_stages(self) + 1 {
            let n_cols = *self.stark_info.map_sections_n.get(&format!("cm{}", stage)).unwrap() as usize;
            if n_cols > max_cols {
                max_cols = n_cols;
            }
        }

        let n_extended = (1 << self.stark_info.stark_struct.n_bits_ext) as usize;
        let buff_size_stages = max_cols * n_extended;

        let buff_size_xdivxsub = self.stark_info.opening_points.len() * 3 * n_extended;

        match buff_size_stages > buff_size_xdivxsub {
            true => buff_size_stages,
            false => buff_size_xdivxsub,
        }
    }

    fn calculate_hash(&self, values: Vec<F>) -> Vec<F> {
        let hash = vec![F::zero(); self.n_field_elements];
        calculate_hash_c(
            self.p_stark,
            hash.as_ptr() as *mut c_void,
            values.as_ptr() as *mut c_void,
            values.len() as u64,
        );
        hash
    }

    fn get_transcript_values(&self, stage: u64, proof_ctx: Arc<ProofCtx<F>>) -> Vec<F> {
        let values =
            self.get_transcript_values_u64(stage, proof_ctx).iter().map(|v| F::from_canonical_u64(*v)).collect();
        values
    }

    fn get_transcript_values_u64(&self, stage: u64, proof_ctx: Arc<ProofCtx<F>>) -> Vec<u64> {
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;

        let mut value = vec![Goldilocks::zero(); self.n_field_elements];
        if stage <= (Self::num_stages(self) + 1) as u64 {
            let (n_airvals_stage, indexes): (usize, Vec<usize>) = self
                .stark_info
                .airvalues_map
                .as_ref()
                .map(|map| {
                    let mut indexes = Vec::new();
                    let count = map
                        .iter()
                        .enumerate()
                        .filter(|(index, entry)| {
                            if entry.stage == stage {
                                indexes.push(*index);
                                true
                            } else {
                                false
                            }
                        })
                        .count();

                    (count, indexes)
                })
                .unwrap_or((0, Vec::new()));

            if stage == 1 || n_airvals_stage > 0 {
                let size = if stage == 1 {
                    2 * self.n_field_elements + n_airvals_stage
                } else {
                    self.n_field_elements + n_airvals_stage * Self::FIELD_EXTENSION
                };
                let mut values_hash = vec![F::zero(); size];

                let verkey = proof_ctx
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
                treesGL_get_root_c(p_stark, stage - 1, root.as_mut_ptr() as *mut c_void);

                trace!(
                    "{}: ··· MerkleTree root for stage {} of instance {} of {} is: {:?}",
                    Self::MY_NAME,
                    stage,
                    self.instance_id,
                    air_name,
                    root,
                );
                for (j, &root_value) in root.iter().enumerate().take(self.n_field_elements) {
                    let index = if stage == 1 { self.n_field_elements + j } else { j };
                    values_hash[index] = root_value;
                }
                let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
                for index in 0..n_airvals_stage {
                    if stage == 1 {
                        values_hash[2 * self.n_field_elements + index] =
                            air_instance.airvalues[indexes[index] * Self::FIELD_EXTENSION];
                    } else {
                        values_hash[self.n_field_elements + index * Self::FIELD_EXTENSION] =
                            air_instance.airvalues[indexes[index] * Self::FIELD_EXTENSION];
                        values_hash[self.n_field_elements + index * Self::FIELD_EXTENSION + 1] =
                            air_instance.airvalues[indexes[index] * Self::FIELD_EXTENSION];
                        values_hash[self.n_field_elements + index * Self::FIELD_EXTENSION + 2] =
                            air_instance.airvalues[indexes[index] * Self::FIELD_EXTENSION];
                    }
                }

                calculate_hash_c(
                    p_stark,
                    value.as_mut_ptr() as *mut c_void,
                    values_hash.as_mut_ptr() as *mut c_void,
                    size as u64,
                );
            } else {
                treesGL_get_root_c(p_stark, stage - 1, value.as_mut_ptr() as *mut c_void);
            }
        } else if stage == (Self::num_stages(self) + 2) as u64 {
            let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
            let evals = air_instance.evals.as_ptr() as *mut c_void;
            calculate_hash_c(
                p_stark,
                value.as_mut_ptr() as *mut c_void,
                evals,
                (self.stark_info.ev_map.len() * Self::FIELD_EXTENSION) as u64,
            );
        } else if stage > (Self::num_stages(self) + 3) as u64 {
            let steps = &self.stark_info.stark_struct.steps;

            let steps_fri: Vec<usize> = proof_ctx.global_info.steps_fri.iter().map(|step| step.n_bits).collect();
            let step_index =
                self.stark_info.stark_struct.steps.iter().position(|s| {
                    s.n_bits as usize == steps_fri[(stage as u32 - (Self::num_stages(self) + 4)) as usize]
                });

            if let Some(step_index) = step_index {
                let n_steps = steps.len() - 1;
                if step_index < n_steps {
                    let p_proof = self.p_proof;
                    fri_proof_get_tree_root_c(p_proof, value.as_mut_ptr() as *mut c_void, step_index as u64);
                } else {
                    let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
                    let buffer = air_instance.get_buffer_ptr() as *mut c_void;

                    let n_hash = (1 << (steps[n_steps].n_bits)) * Self::FIELD_EXTENSION as u64;
                    let fri_pol = get_fri_pol_c(self.p_stark_info, buffer);
                    calculate_hash_c(p_stark, value.as_mut_ptr() as *mut c_void, fri_pol, n_hash);
                }
            }
        }
        let mut value64: Vec<u64> = Vec::new();
        for v in value {
            value64.push(v.as_canonical_u64());
        }
        value64
    }

    fn get_challenges(&self, stage_id: u32, proof_ctx: Arc<ProofCtx<F>>, transcript: &FFITranscript) {
        if stage_id == 1 {
            return;
        }

        if stage_id <= self.num_stages() + 3 {
            //num stages + 1 + evals + fri_pol (then starts fri folding...)

            let challenges_map = self.stark_info.challenges_map.as_ref().unwrap();

            let challenges = &*proof_ctx.challenges.challenges.read().unwrap();
            for i in 0..challenges_map.len() {
                if challenges_map[i].stage == stage_id as u64 {
                    let challenge = &challenges[i * Self::FIELD_EXTENSION];
                    transcript.get_challenge(challenge as *const F as *mut c_void);
                    debug!(
                        "{}: ··· Global challenge: [{}, {}, {}]",
                        Self::MY_NAME,
                        challenges[i * Self::FIELD_EXTENSION],
                        challenges[i * Self::FIELD_EXTENSION + 1],
                        challenges[i * Self::FIELD_EXTENSION + 2],
                    );
                }
            }
        } else {
            //Fri folding + . queries: add one challenge for each step
            let mut challenges_guard = proof_ctx.challenges.challenges.write().unwrap();

            challenges_guard.extend(std::iter::repeat(F::zero()).take(3));
            transcript.get_challenge(&(*challenges_guard)[challenges_guard.len() - 3] as *const F as *mut c_void);
            debug!(
                "{}: ··· Global challenge: [{}, {}, {}]",
                Self::MY_NAME,
                challenges_guard[challenges_guard.len() - 3],
                challenges_guard[challenges_guard.len() - 2],
                challenges_guard[challenges_guard.len() - 1],
            );
        }
    }

    fn get_proof(&self) -> *mut c_void {
        self.p_proof
    }

    fn get_zkin_proof(&self, proof_ctx: Arc<ProofCtx<F>>, output_dir: &str) -> *mut c_void {
        let gidx = proof_ctx.air_instance_repo.air_instances.read().unwrap()[self.prover_idx].global_idx.unwrap();
        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;

        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();
        let challenges = (*challenges_guard).as_ptr() as *mut c_void;

        let global_info_path = proof_ctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
        let global_info_file: &str = global_info_path.to_str().unwrap();

        fri_proof_get_zkinproof_c(
            gidx as u64,
            self.p_proof,
            public_inputs,
            challenges,
            self.p_stark_info,
            global_info_file,
            output_dir,
        )
    }

    fn get_prover_info(&self) -> ProverInfo {
        ProverInfo {
            airgroup_id: self.airgroup_id,
            air_id: self.air_id,
            prover_idx: self.prover_idx,
            instance_id: self.instance_id,
        }
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
    fn compute_evals(&mut self, _opening_id: u32, setup_ctx: Arc<SetupCtx>, proof_ctx: Arc<ProofCtx<F>>) {
        let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;
        debug!("{}: ··· Calculating evals of instance {} of {}", Self::MY_NAME, self.instance_id, air_name);
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let buffer = air_instance.get_buffer_ptr() as *mut c_void;

        let evals = air_instance.evals.as_mut_ptr() as *mut c_void;

        let setup = setup_ctx.get_setup(self.airgroup_id, self.air_id);

        let p_stark = self.p_stark;
        let p_proof = self.p_proof;

        let buff_helper_guard = proof_ctx.buff_helper.buff_helper.read().unwrap();
        let buff_helper = (*buff_helper_guard).as_ptr() as *mut c_void;

        let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

        let steps_params = StepsParams {
            buffer,
            public_inputs: std::ptr::null_mut(),
            challenges: std::ptr::null_mut(),
            airgroup_values: std::ptr::null_mut(),
            airvalues: std::ptr::null_mut(),
            evals,
            xdivxsub: std::ptr::null_mut(),
            p_const_pols: std::ptr::null_mut(),
            p_const_tree: const_tree_ptr,
            custom_commits: air_instance.get_custom_commits_ptr(),
        };

        compute_evals_c(p_stark, (&steps_params).into(), buff_helper, p_proof);
    }

    fn compute_fri_pol(&mut self, _opening_id: u32, setup_ctx: Arc<SetupCtx>, proof_ctx: Arc<ProofCtx<F>>) {
        let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;
        debug!("{}: ··· Calculating FRI polynomial of instance {} of {}", Self::MY_NAME, self.instance_id, air_name);
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let setup = setup_ctx.get_setup(self.airgroup_id, self.air_id);

        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();
        let buff_helper_guard = proof_ctx.buff_helper.buff_helper.read().unwrap();

        let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
        let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

        let p_stark = self.p_stark;

        let steps_params = StepsParams {
            buffer: air_instance.get_buffer_ptr() as *mut c_void,
            public_inputs: (*public_inputs_guard).as_ptr() as *mut c_void,
            challenges: (*challenges_guard).as_ptr() as *mut c_void,
            airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
            airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
            evals: air_instance.evals.as_ptr() as *mut c_void,
            xdivxsub: (*buff_helper_guard).as_ptr() as *mut c_void,
            p_const_pols: const_pols_ptr,
            p_const_tree: const_tree_ptr,
            custom_commits: air_instance.get_custom_commits_ptr(),
        };

        calculate_fri_polynomial_c(p_stark, (&steps_params).into());
    }

    fn compute_fri_folding(&mut self, step_index: u32, proof_ctx: Arc<ProofCtx<F>>) {
        let p_proof = self.p_proof;

        let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;

        let steps = &self.stark_info.stark_struct.steps;
        let n_steps = (steps.len() - 1) as u32;
        if step_index == n_steps {
            debug!(
                "{}: ··· Calculating final FRI polynomial of instance {} of {}",
                Self::MY_NAME,
                self.instance_id,
                air_name
            );
        } else {
            debug!("{}: ··· Calculating FRI folding of instance {} of {}", Self::MY_NAME, self.instance_id, air_name);
        }

        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
        let buffer = air_instance.get_buffer_ptr() as *mut c_void;

        let fri_pol = get_fri_pol_c(self.p_stark_info, buffer);

        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();
        let challenge: Vec<F> = challenges_guard.iter().skip(challenges_guard.len() - 3).cloned().collect();

        let current_bits = steps[step_index as usize].n_bits;
        let prev_bits = if step_index == 0 { current_bits } else { steps[(step_index - 1) as usize].n_bits };

        compute_fri_folding_c(
            step_index as u64,
            fri_pol,
            challenge.as_ptr() as *mut c_void,
            self.stark_info.stark_struct.n_bits_ext,
            prev_bits,
            current_bits,
        );

        if step_index != n_steps {
            let next_bits = steps[(step_index + 1) as usize].n_bits;
            compute_fri_merkelize_c(self.p_stark, p_proof, step_index as u64, fri_pol, current_bits, next_bits);
        }
    }

    fn compute_fri_queries(&mut self, _opening_id: u32, proof_ctx: Arc<ProofCtx<F>>) {
        let p_stark = self.p_stark;
        let p_proof = self.p_proof;

        let n_queries = self.stark_info.stark_struct.n_queries;
        let steps = &self.stark_info.stark_struct.steps;
        let air_name = &proof_ctx.global_info.airs[self.airgroup_id][self.air_id].name;
        debug!("{}: ··· Calculating FRI queries of instance {} of {}", Self::MY_NAME, self.instance_id, air_name);

        let mut fri_queries = vec![u64::default(); n_queries as usize];

        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let challenge: Vec<F> = challenges_guard.iter().skip(challenges_guard.len() - 3).cloned().collect();

        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };
        let transcript_permutation =
            FFITranscript::new(p_stark, element_type, self.merkle_tree_arity, self.merkle_tree_custom);

        transcript_permutation.add_elements(challenge.as_ptr() as *mut c_void, Self::FIELD_EXTENSION);
        transcript_permutation.get_permutations(
            fri_queries.as_mut_ptr(),
            n_queries,
            self.stark_info.stark_struct.steps[0].n_bits,
        );

        trace!(
            "{}: ··· FRI queries of instance {} of {} are: {:?}",
            Self::MY_NAME,
            self.instance_id,
            air_name,
            &fri_queries,
        );

        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
        let buffer = air_instance.get_buffer_ptr() as *mut c_void;

        let fri_pol = get_fri_pol_c(self.p_stark_info, buffer);

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
            fri_pol,
            self.stark_info.stark_struct.steps[self.stark_info.stark_struct.steps.len() - 1].n_bits,
        );
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
    fn get_buffer_info(
        &self,
        sctx: &SetupCtx,
        airgroup_id: usize,
        air_id: usize,
    ) -> Result<(u64, Vec<u64>), Box<dyn Error>> {
        let ps = sctx.get_setup(airgroup_id, air_id);

        let p_stark_info = ps.p_setup.p_stark_info;
        Ok((get_map_totaln_c(p_stark_info), vec![get_map_offsets_c(p_stark_info, "cm1", false)]))
    }

    fn get_buffer_info_custom_commit(
        &self,
        sctx: &SetupCtx,
        airgroup_id: usize,
        air_id: usize,
        name: &str,
    ) -> Result<(u64, Vec<u64>, u64), Box<dyn Error>> {
        let ps = sctx.get_setup(airgroup_id, air_id);

        let commit_id = match ps.stark_info.custom_commits.iter().position(|custom_commit| custom_commit.name == name) {
            Some(commit_id) => commit_id as u64,
            None => {
                eprintln!("Custom commit '{}' not found in custom commits.", name);
                std::process::exit(1);
            }
        };
        Ok((get_map_totaln_custom_commits_c(ps.p_setup.p_stark_info, commit_id), vec![0], commit_id))
    }
}
