use std::error::Error;
use std::path::PathBuf;

use std::any::type_name;
use std::sync::Arc;

use proofman_common::{
    BufferAllocator, ConstraintInfo, ConstraintsResults, ProofCtx, ProofType, Prover, ProverInfo, ProverStatus,
    SetupCtx,
};
use log::debug;
use transcript::FFITranscript;
use proofman_util::{timer_start, timer_stop_and_log};
use proofman_starks_lib_c::*;
use crate::stark_info::StarkInfo;
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
    p_setup: *mut c_void,
    pub p_stark: *mut c_void,
    p_stark_info: *mut c_void,
    stark_info: StarkInfo,
    n_field_elements: usize,
    merkle_tree_arity: u64,
    merkle_tree_custom: bool,
    p_proof: Option<*mut c_void>,
    global_steps_fri: Vec<usize>,
    global_n_stages: usize,
    _marker: PhantomData<F>, // Add PhantomData to track the type F
}

impl<F: Field> StarkProver<F> {
    const MY_NAME: &'static str = "estrkPrv";

    const HASH_SIZE: usize = 4;
    const FIELD_EXTENSION: usize = 3;

    pub fn new(
        sctx: Arc<SetupCtx>,
        pctx: Arc<ProofCtx<F>>,
        airgroup_id: usize,
        air_id: usize,
        prover_idx: usize,
    ) -> Self {
        let air_setup_path = pctx.global_info.get_air_setup_path(airgroup_id, air_id, &ProofType::Basic);

        let setup = sctx.get_setup(airgroup_id, air_id).expect("REASON");

        let p_stark = starks_new_c((&setup.p_setup).into());

        let stark_info_path = air_setup_path.display().to_string() + ".starkinfo.json";
        let stark_info_json = std::fs::read_to_string(&stark_info_path)
            .unwrap_or_else(|_| panic!("Failed to read file {}", &stark_info_path));
        let stark_info: StarkInfo = StarkInfo::from_json(&stark_info_json);

        let (n_field_elements, merkle_tree_arity, merkle_tree_custom) =
            if stark_info.stark_struct.verification_hash_type == "BN128" {
                (1, stark_info.stark_struct.merkle_tree_arity, stark_info.stark_struct.merkle_tree_custom)
            } else {
                (Self::HASH_SIZE, 2, true)
            };

        let global_steps_fri: Vec<usize> = pctx.global_info.steps_fri.iter().map(|step| step.n_bits).collect();
        let global_n_stages = pctx.global_info.n_challenges.len();

        Self {
            initialized: true,
            prover_idx,
            air_id,
            airgroup_id,
            p_setup: (&setup.p_setup).into(),
            p_stark_info: setup.p_setup.p_stark_info,
            p_stark,
            p_proof: None,
            stark_info,
            n_field_elements,
            merkle_tree_arity,
            merkle_tree_custom,
            global_steps_fri,
            global_n_stages,
            _marker: PhantomData,
        }
    }
}

impl<F: Field> Prover<F> for StarkProver<F> {
    fn build(&mut self, proof_ctx: Arc<ProofCtx<F>>) {
        timer_start!(ESTARK_PROVER_BUILD);
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        //initialize the common challenges if have not been initialized by another prover
        let challenges =
            vec![F::zero(); self.stark_info.challenges_map.as_ref().unwrap().len() * Self::FIELD_EXTENSION];
        *proof_ctx.challenges.challenges.write().unwrap() = challenges;

        let n_subproof_values = self.stark_info.subproofvalues_map.as_ref().expect("REASON").len();
        let n_evals = self.stark_info.ev_map.len();

        let evals = vec![F::zero(); n_evals * Self::FIELD_EXTENSION];
        let subproof_values = vec![F::zero(); n_subproof_values * Self::FIELD_EXTENSION];

        air_instance.init_prover(evals, subproof_values);

        self.p_proof = Some(fri_proof_new_c(self.p_setup));

        let number_stage1_commits = *self.stark_info.map_sections_n.get("cm1").unwrap() as usize;
        for i in 0..number_stage1_commits {
            air_instance.set_commit_calculated(i);
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

    fn verify_constraints(&self, proof_ctx: Arc<ProofCtx<F>>) -> Vec<ConstraintInfo> {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
        let challenges = (*challenges_guard).as_ptr() as *mut c_void;

        let buffer = air_instance.get_buffer_ptr() as *mut c_void;
        let evals = air_instance.evals.as_ptr() as *mut c_void;
        let subproof_values = air_instance.subproof_values.as_ptr() as *mut c_void;

        let raw_ptr = verify_constraints_c(self.p_setup, buffer, public_inputs, challenges, subproof_values, evals);

        unsafe {
            let constraints_result = Box::from_raw(raw_ptr as *mut ConstraintsResults);
            std::slice::from_raw_parts(constraints_result.constraints_info, constraints_result.n_constraints as usize)
        }
        .to_vec()
    }

    fn calculate_stage(&mut self, stage_id: u32, proof_ctx: Arc<ProofCtx<F>>) {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let buffer = air_instance.get_buffer_ptr() as *mut c_void;
        let evals = air_instance.evals.as_ptr() as *mut c_void;
        let subproof_values = air_instance.subproof_values.as_ptr() as *mut c_void;

        let n_commits = self.stark_info.cm_pols_map.as_ref().expect("REASON").len();

        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
        let challenges = (*challenges_guard).as_ptr() as *mut c_void;

        if stage_id as usize <= self.global_n_stages {
            for i in 0..n_commits {
                let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                if (cm_pol.stage < stage_id as u64 || cm_pol.stage == stage_id as u64 && !cm_pol.im_pol)
                    && !air_instance.commits_calculated.contains_key(&i)
                {
                    panic!("Intermediate polynomials for stage {} cannot be calculated: Witness column {} is not calculated", stage_id, cm_pol.name);
                }
            }
            calculate_impols_expressions_c(
                self.p_stark,
                stage_id as u64,
                buffer,
                public_inputs,
                challenges,
                subproof_values,
                evals,
            );
            for i in 0..n_commits {
                let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                if cm_pol.stage == stage_id as u64 && cm_pol.im_pol {
                    air_instance.set_commit_calculated(i);
                }
            }
            if stage_id as usize == self.global_n_stages {
                let p_proof = self.p_proof.unwrap();
                fri_proof_set_subproof_values_c(p_proof, subproof_values);
            }
        } else {
            calculate_quotient_polynomial_c(self.p_stark, buffer, public_inputs, challenges, subproof_values, evals);
            for i in 0..n_commits {
                let cm_pol: &crate::stark_info::PolMap =
                    self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
                if cm_pol.stage == (self.global_n_stages + 1) as u64 {
                    air_instance.set_commit_calculated(i);
                }
            }
        }
    }

    fn commit_stage(&mut self, stage_id: u32, proof_ctx: Arc<ProofCtx<F>>) -> ProverStatus {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
        let buffer = air_instance.get_buffer_ptr() as *mut c_void;
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        debug!("{}: ··· Computing commit stage {}", Self::MY_NAME, stage_id);

        timer_start!(STARK_COMMIT_STAGE_, stage_id);

        let p_proof = self.p_proof.unwrap();
        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };

        let n_commits = self.stark_info.cm_pols_map.as_ref().expect("REASON").len();
        for i in 0..n_commits {
            let cm_pol = self.stark_info.cm_pols_map.as_ref().expect("REASON").get(i).unwrap();
            if cm_pol.stage == stage_id as u64 && !air_instance.commits_calculated.contains_key(&i) {
                panic!("Stage {} cannot be committed: Witness column {} is not calculated", stage_id, cm_pol.name);
            }
        }

        if stage_id == self.num_stages() {
            let n_subproof_values = self.stark_info.subproofvalues_map.as_ref().expect("REASON").len();
            for i in 0..n_subproof_values {
                let subproof_value = self.stark_info.subproofvalues_map.as_ref().expect("REASON").get(i).unwrap();
                if !air_instance.subproofvalue_calculated.contains_key(&i) {
                    panic!(
                        "Stage {} cannot be committed: Subproofvalue {} is not calculated ---> {}",
                        stage_id, subproof_value.name, i
                    );
                }
            }
        }

        let buff_helper_guard = proof_ctx.buff_helper.buff_helper.read().unwrap();
        let buff_helper = (*buff_helper_guard).as_ptr() as *mut c_void;
        commit_stage_c(p_stark, element_type, stage_id as u64, buffer, p_proof, buff_helper);

        timer_stop_and_log!(STARK_COMMIT_STAGE_, stage_id);

        if stage_id <= self.num_stages() + 1 {
            ProverStatus::CommitStage
        } else {
            ProverStatus::OpeningStage
        }
    }

    fn opening_stage(&mut self, opening_id: u32, proof_ctx: Arc<ProofCtx<F>>) -> ProverStatus {
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
                self.compute_fri_folding(step_index as u32, proof_ctx);
            } else {
                debug!("{}: ··· Skipping FRI Folding", Self::MY_NAME,);
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
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        let mut value = vec![F::zero(); self.n_field_elements];
        if stage <= (Self::num_stages(self) + 1) as u64 {
            let tree_index = if stage == 0 {
                let stark_info: &StarkInfo = &self.stark_info;
                stark_info.n_stages as u64 + 1
            } else {
                stage - 1
            };

            treesGL_get_root_c(p_stark, tree_index, value.as_mut_ptr() as *mut c_void);
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

            let global_step_fri = self.global_steps_fri[(stage as u32 - (Self::num_stages(self) + 4)) as usize];
            let step_index =
                self.stark_info.stark_struct.steps.iter().position(|s| s.n_bits as usize == global_step_fri);

            if let Some(step_index) = step_index {
                let n_steps = steps.len() - 1;
                if step_index < n_steps {
                    let p_proof = self.p_proof.unwrap();
                    fri_proof_get_tree_root_c(p_proof, value.as_mut_ptr() as *mut c_void, (step_index + 1) as u64);
                } else {
                    let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
                    let buffer = air_instance.get_buffer_ptr() as *mut c_void;

                    let n_hash = (1 << (steps[n_steps].n_bits)) * Self::FIELD_EXTENSION as u64;
                    let fri_pol = get_fri_pol_c(self.p_setup, buffer);
                    calculate_hash_c(p_stark, value.as_mut_ptr() as *mut c_void, fri_pol, n_hash);
                }
            }
        }
        value
    }

    fn get_transcript_values_u64(&self, stage: u64, proof_ctx: Arc<ProofCtx<F>>) -> Vec<u64> {
        let p_stark: *mut std::ffi::c_void = self.p_stark;

        let mut value: Vec<Goldilocks> = vec![Goldilocks::zero(); self.n_field_elements];
        if stage <= (Self::num_stages(self) + 1) as u64 {
            let tree_index = if stage == 0 {
                let stark_info: &StarkInfo = &self.stark_info;
                stark_info.n_stages as u64 + 1
            } else {
                stage - 1
            };

            treesGL_get_root_c(p_stark, tree_index, value.as_mut_ptr() as *mut c_void);
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

            let global_step_fri = self.global_steps_fri[(stage as u32 - (Self::num_stages(self) + 4)) as usize];
            let step_index =
                self.stark_info.stark_struct.steps.iter().position(|s| s.n_bits as usize == global_step_fri);

            if let Some(step_index) = step_index {
                let n_steps = steps.len() - 1;
                if step_index < n_steps {
                    let p_proof = self.p_proof.unwrap();
                    fri_proof_get_tree_root_c(p_proof, value.as_mut_ptr() as *mut c_void, (step_index + 1) as u64);
                } else {
                    let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
                    let buffer = air_instance.get_buffer_ptr() as *mut c_void;

                    let n_hash = (1 << (steps[n_steps].n_bits)) * Self::FIELD_EXTENSION as u64;
                    let fri_pol = get_fri_pol_c(self.p_setup, buffer);
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
                }
            }
        } else {
            //Fri folding + . queries: add one challenge for each step
            let mut challenges_guard = proof_ctx.challenges.challenges.write().unwrap();

            challenges_guard.extend(std::iter::repeat(F::zero()).take(3));
            transcript.get_challenge(&(*challenges_guard)[challenges_guard.len() - 3] as *const F as *mut c_void);
        }
    }

    fn get_proof(&self) -> *mut c_void {
        self.p_proof.unwrap()
    }

    fn save_proof(&self, proof_ctx: Arc<ProofCtx<F>>, output_dir: &str, save_json: bool) -> *mut c_void {
        let idx = self.prover_idx;
        #[cfg(feature = "distributed")]
        {
            let segment_id: &usize =
                &proof_ctx.air_instance_repo.air_instances.read().unwrap()[self.prover_idx].air_segment_id.unwrap();
            idx = *segment_id;
        }
        if save_json {
            save_proof_c(idx as u64, self.p_stark_info, self.p_proof.unwrap(), output_dir);
        }

        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;

        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();
        let challenges = (*challenges_guard).as_ptr() as *mut c_void;

        let global_info_path = proof_ctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
        let global_info_file: &str = global_info_path.to_str().unwrap();

        let output_json_dir = if save_json { output_dir } else { "" };

        fri_proof_get_zkinproof_c(
            idx as u64,
            self.p_proof.unwrap(),
            public_inputs,
            challenges,
            self.p_stark_info,
            global_info_file,
            output_json_dir,
        )
    }

    fn get_prover_info(&self) -> ProverInfo {
        ProverInfo { airgroup_id: self.airgroup_id, air_id: self.air_id, prover_idx: self.prover_idx }
    }
}

impl<F: Field> StarkProver<F> {
    // Return the total number of elements needed to compute the STARK
    pub fn get_total_bytes(&self) -> usize {
        get_map_totaln_c(self.p_setup) as usize * std::mem::size_of::<F>()
    }

    fn compute_evals(&mut self, _opening_id: u32, proof_ctx: Arc<ProofCtx<F>>) {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let buffer = air_instance.get_buffer_ptr() as *mut c_void;

        let evals = air_instance.evals.as_mut_ptr() as *mut c_void;

        let p_stark = self.p_stark;
        let p_proof = self.p_proof.unwrap();

        debug!("{}: ··· Computing evaluations", Self::MY_NAME);

        let buff_helper_guard = proof_ctx.buff_helper.buff_helper.read().unwrap();
        let buff_helper = (*buff_helper_guard).as_ptr() as *mut c_void;

        compute_evals_c(p_stark, buffer, buff_helper, evals, p_proof);
    }

    fn compute_fri_pol(&mut self, _opening_id: u32, proof_ctx: Arc<ProofCtx<F>>) {
        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];

        let buffer = air_instance.get_buffer_ptr() as *mut c_void;
        let public_inputs_guard = proof_ctx.public_inputs.inputs.read().unwrap();
        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
        let challenges = (*challenges_guard).as_ptr() as *mut c_void;

        let evals = air_instance.evals.as_ptr() as *mut c_void;
        let subproof_values = air_instance.subproof_values.as_ptr() as *mut c_void;

        let p_stark = self.p_stark;

        debug!("{}: ··· Computing FRI Polynomial", Self::MY_NAME);

        let buff_helper_guard = proof_ctx.buff_helper.buff_helper.read().unwrap();
        let xdivxsub = (*buff_helper_guard).as_ptr() as *mut c_void;

        calculate_fri_polynomial_c(p_stark, buffer, public_inputs, challenges, subproof_values, evals, xdivxsub);
    }

    fn compute_fri_folding(&mut self, step_index: u32, proof_ctx: Arc<ProofCtx<F>>) {
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

        let air_instance = &mut proof_ctx.air_instance_repo.air_instances.write().unwrap()[self.prover_idx];
        let buffer = air_instance.get_buffer_ptr() as *mut c_void;

        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();
        let challenge: Vec<F> = challenges_guard.iter().skip(challenges_guard.len() - 3).cloned().collect();

        compute_fri_folding_c(p_stark, step_index as u64, p_proof, buffer, challenge.as_ptr() as *mut c_void);
    }

    fn compute_fri_queries(&mut self, _opening_id: u32, proof_ctx: Arc<ProofCtx<F>>) {
        let p_stark = self.p_stark;
        let p_proof = self.p_proof.unwrap();

        debug!("{}: ··· Computing FRI queries", Self::MY_NAME);

        let mut fri_queries = vec![u64::default(); self.stark_info.stark_struct.n_queries as usize];

        let challenges_guard = proof_ctx.challenges.challenges.read().unwrap();

        let challenge: Vec<F> = challenges_guard.iter().skip(challenges_guard.len() - 3).cloned().collect();

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
    fn get_buffer_info(
        &self,
        sctx: &SetupCtx,
        airgroup_id: usize,
        air_id: usize,
    ) -> Result<(u64, Vec<u64>), Box<dyn Error>> {
        let ps = sctx.get_partial_setup(airgroup_id, air_id).expect("REASON");

        Ok((get_map_totaln_c(ps.p_setup.p_stark_info), vec![get_map_offsets_c(ps.p_setup.p_stark_info, "cm1", false)]))
    }
}
