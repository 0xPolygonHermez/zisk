use goldilocks::{AbstractField, Goldilocks};
use proofman::proof_manager::ProverStatus;
use transcript::FFITranscript;
use std::any::type_name;

use proofman::provers_manager::Prover;
use log::debug;
use util::{timer_start, timer_stop_and_log};
use zkevm_lib_c::ffi::*;
use proofman::proof_ctx::ProofCtx;
use crate::stark_info::StarkInfo;
use crate::stark_prover_settings::StarkProverSettings;

use std::os::raw::c_void;

pub struct StarkProver<T: AbstractField> {
    initialized: bool,
    config: StarkProverSettings,
    p_steps: *mut c_void,
    ptr: *mut c_void,
    pub p_stark: Option<*mut c_void>,
    p_params: Option<*mut c_void>,
    p_challenges: Option<*mut c_void>,
    p_evals: Option<*mut c_void>,
    p_proof: Option<*mut c_void>,
    transcript: Option<FFITranscript>,
    p_fri_pol: Option<*mut c_void>,
    stark_info: Option<StarkInfo>,
    phantom: std::marker::PhantomData<T>,
}

impl<T: AbstractField> StarkProver<T> {
    const MY_NAME: &'static str = "estrkPrv";

    const HASH_SIZE: u64 = 4;
    const FIELD_EXTENSION: u64 = 3;

    pub fn new(config: StarkProverSettings, p_steps: *mut c_void, ptr: *mut c_void) -> Self {
        Self {
            initialized: false,
            config,
            p_steps,
            ptr,
            p_stark: None,
            p_params: None,
            p_challenges: None,
            p_evals: None,
            p_proof: None,
            transcript: None,
            p_fri_pol: None,
            stark_info: None,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn get_stark_info(&self) -> *mut c_void {
        get_stark_info_c(self.p_stark.unwrap())
    }
}

impl<T: AbstractField> Prover<T> for StarkProver<T> {
    fn build(&mut self) {
        timer_start!(ESTARK_PROVER_NEW);

        let p_config = config_new_c(&self.config.current_path);
        let stark_info_json = std::fs::read_to_string(&self.config.stark_info_filename)
            .expect(format!("Failed to read file {}", &self.config.stark_info_filename).as_str());
        self.stark_info = Some(StarkInfo::from_json(&stark_info_json));

        let p_stark = starks_new_c(
            p_config,
            self.config.const_pols_filename.as_str(),
            self.config.map_const_pols_file,
            self.config.const_tree_filename.as_str(),
            self.config.stark_info_filename.as_str(),
            self.config.chelpers_filename.as_str(),
            self.ptr,
        );

        self.p_stark = Some(p_stark);

        let element_type = if type_name::<T>() == type_name::<Goldilocks>() { 1 } else { 0 };
        self.transcript = Some(FFITranscript::new(p_stark, element_type));

        self.initialized = true;

        timer_stop_and_log!(ESTARK_PROVER_NEW);
    }

    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        debug!("{}: ··· Computing commit stage {}", Self::MY_NAME, stage_id);

        if !self.initialized {
            self.build();
        }

        let transcript = self.transcript.as_ref().unwrap();
        let p_stark = self.p_stark.unwrap();

        let element_type = if type_name::<T>() == type_name::<Goldilocks>() { 1 } else { 0 };

        if stage_id == 1 {
            const HASH_SIZE: u64 = 4;
            const FIELD_EXTENSION: u64 = 3;

            timer_start!(STARK_INITIALIZATION);

            let stark_info = self.stark_info.as_ref().unwrap();
            let n_stages = stark_info.n_stages.unwrap();

            let p_evals = polinomial_new_c(stark_info.ev_map.len() as u64, FIELD_EXTENSION, "");
            self.p_evals = Some(p_evals);
            let p_challenges = polinomial_new_c(stark_info.n_challenges.unwrap(), FIELD_EXTENSION, "");
            self.p_challenges = Some(p_challenges);

            let p_subproof_values = polinomial_new_c(stark_info.n_subair_values, FIELD_EXTENSION, "");

            let n_extended = 1 << stark_info.stark_struct.n_bits_ext;
            let p_x_div_x_sub_xi =
                polinomial_new_c(stark_info.opening_points.len() as u64 * n_extended, FIELD_EXTENSION, "");

            let hash_size = if stark_info.stark_struct.verification_hash_type == "BN128" { 1 } else { HASH_SIZE };
            let verkey = vec![T::zero(); hash_size as usize];

            treesGL_get_root_c(p_stark, n_stages + 1, verkey.as_ptr() as *mut c_void);

            self.p_proof = Some(fri_proof_new_c(p_stark));

            self.p_params = Some(steps_params_new_c(
                p_stark,
                p_challenges,
                p_subproof_values,
                p_evals,
                p_x_div_x_sub_xi,
                proof_ctx.public_inputs.as_ptr() as *mut c_void,
            ));

            timer_stop_and_log!(STARK_INITIALIZATION);

            //--------------------------------
            // 0.- Add const root and publics to transcript
            //--------------------------------
            timer_start!(STARK_COMMIT_STAGE_0);

            transcript.add_elements(verkey.as_ptr() as *mut c_void, HASH_SIZE);
            transcript.add_elements(proof_ctx.public_inputs.as_ptr() as *mut c_void, stark_info.n_publics);

            timer_stop_and_log!(STARK_COMMIT_STAGE_0);
        }

        let p_params = self.p_params.unwrap();
        let p_proof = self.p_proof.unwrap();

        timer_start!(STARK_COMMIT_STAGE_, stage_id);
        compute_stage_c(
            p_stark,
            element_type,
            stage_id as u64,
            p_params,
            p_proof,
            transcript.p_transcript,
            self.p_steps,
        );
        timer_stop_and_log!(STARK_COMMIT_STAGE_, stage_id);

        ProverStatus::StagesPending
    }

    fn opening_stage(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        let last_stage_id = 2 + self.stark_info.as_ref().unwrap().stark_struct.steps.len() as u32 + 1;

        if opening_id == 1 {
            self.compute_evals(opening_id, proof_ctx);
        } else if opening_id == 2 {
            self.compute_fri_pol(opening_id, proof_ctx);
        } else if opening_id < last_stage_id {
            self.compute_fri_folding(opening_id, proof_ctx);
        } else if opening_id == last_stage_id {
            self.compute_fri_queries(opening_id, proof_ctx);
        } else {
            panic!("Opening stage not implemented");
        }

        if opening_id == last_stage_id {
            ProverStatus::StagesCompleted
        } else {
            ProverStatus::StagesPending
        }
    }
}

impl<T: AbstractField> StarkProver<T> {
    fn compute_evals(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<T>) {
        let p_stark = self.p_stark.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_params = self.p_params.unwrap();
        let p_proof = self.p_proof.unwrap();
        let transcript = self.transcript.as_ref().unwrap();
        let p_challenges = self.p_challenges.unwrap();
        let p_evals = self.p_evals.unwrap();

        debug!("{}: ··· Computing evaluations", Self::MY_NAME);

        let mut challenge_index = stark_info.num_challenges.iter().sum::<u64>() + 1;

        if stark_info.pil2 {
            // in a future version, this will be removed because all will be pil2 While developing this will be kept
            transcript.get_challenge(polinomial_get_p_element_c(p_challenges, challenge_index));
            challenge_index += 1;
        } else {
            transcript.get_challenge(polinomial_get_p_element_c(p_challenges, 7));
        }
        challenge_index += 1;

        compute_evals_c(p_stark, p_params, p_proof);

        transcript.transcript_add_polinomial(p_evals);

        if stark_info.pil2 {
            // in a future version, this will be removed because all will be pil2. While developing this will be kept
            transcript.get_challenge(polinomial_get_p_element_c(p_challenges, challenge_index));
            challenge_index += 1;
            transcript.get_challenge(polinomial_get_p_element_c(p_challenges, challenge_index));
        } else {
            transcript.get_challenge(polinomial_get_p_element_c(p_challenges, 5));
            transcript.get_challenge(polinomial_get_p_element_c(p_challenges, 6));
        }
    }

    fn compute_fri_pol(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<T>) {
        let p_stark = self.p_stark.unwrap();
        let p_params = self.p_params.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let n_stages = stark_info.n_stages.unwrap();

        debug!("{}: ··· Computing FRI Polynomial", Self::MY_NAME);

        self.p_fri_pol = Some(compute_fri_pol_c(p_stark, n_stages + 2, p_params, self.p_steps));
    }

    fn compute_fri_folding(&mut self, opening_id: u32, _proof_ctx: &mut ProofCtx<T>) {
        let p_stark = self.p_stark.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_proof = self.p_proof.unwrap();
        let transcript = self.transcript.as_ref().unwrap();
        let p_fri_pol = self.p_fri_pol.unwrap();
        let step = opening_id - 3;

        debug!("{}: ··· Computing FRI folding", Self::MY_NAME);

        //TODO! step = Restar opening_id de n_stages + 2
        let challenge = polinomial_new_c(1, Self::FIELD_EXTENSION, "");
        transcript.get_challenge(polinomial_get_p_element_c(challenge, 0));

        compute_fri_folding_c(p_stark, p_proof, p_fri_pol, step as u64, challenge);
        if step < stark_info.stark_struct.steps.len() as u32 - 1 {
            let root = fri_proof_get_tree_root_c(p_proof, step as u64 + 1, 0);
            transcript.add_elements(root, Self::HASH_SIZE);
        } else {
            transcript.transcript_add_polinomial(p_fri_pol);
        }
    }

    fn compute_fri_queries(&mut self, _opening_id: u32, proof_ctx: &mut ProofCtx<T>) {
        let p_stark = self.p_stark.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_proof = self.p_proof.unwrap();
        let transcript = self.transcript.as_ref().unwrap();
        let p_fri_pol = self.p_fri_pol.unwrap();

        debug!("{}: ··· Computing FRI queries", Self::MY_NAME);

        let mut fri_queries = vec![0u64; stark_info.stark_struct.n_queries as usize];

        transcript.get_permutations(
            fri_queries.as_mut_ptr(),
            stark_info.stark_struct.n_queries,
            stark_info.stark_struct.steps[0].n_bits,
        );

        compute_fri_queries_c(p_stark, p_proof, p_fri_pol, fri_queries.as_mut_ptr());

        proof_ctx.proof = Some(p_proof);
    }
}
