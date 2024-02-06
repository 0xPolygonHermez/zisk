use goldilocks::Goldilocks;
use proofman::prover::Prover;
use log::{info, debug};
use util::{timer_start, timer_stop_and_log};
use crate::ffi::*;
use proofman::proof_ctx::ProofCtx;
use crate::verification_key::VerificationKey;
use std::time::Instant;
use crate::stark_info::StarkInfo;

pub struct EStarkProverConfig {
    pub current_path: String,
    pub zkevm_const_pols: String,
    pub map_const_pols_file: bool,
    pub zkevm_constants_tree: String,
    pub zkevm_stark_info: String,
    pub zkevm_verkey: String,
    pub zkevm_verifier: String,
    pub c12a_const_pols: String,
    pub c12a_constants_tree: String,
    pub c12a_stark_info: String,
    pub c12a_verkey: String,
    pub c12a_exec: String,
    pub recursive1_const_pols: String,
    pub recursive1_constants_tree: String,
    pub recursive1_stark_info: String,
    pub recursive1_verkey: String,
    pub recursive1_verifier: String,
    pub recursive1_exec: String,
    pub recursive2_verkey: String,
    // pub save_output_to_file: bool,
    // pub save_proof_to_file: bool,
}

pub struct EStarkProver<T> {
    config: EStarkProverConfig,
    phantom: std::marker::PhantomData<T>,
}

impl<T> EStarkProver<T> {
    const MY_NAME: &'static str = "estarkpr";

    pub fn new(config: EStarkProverConfig) -> Self {
        Self { config, phantom: std::marker::PhantomData }
    }
}

impl<T: Clone> Prover<T> for EStarkProver<T> {
    fn compute_stage(&self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) {
        info!("{}: --> eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        let p_config = config_new_c(&self.config.current_path);

        // Prepare starks to generate the proof
        let p_starks = starks_new_c(
            p_config,
            self.config.zkevm_const_pols.as_str(),
            self.config.map_const_pols_file,
            self.config.zkevm_constants_tree.as_str(),
            self.config.zkevm_stark_info.as_str(),
            proof_ctx.ptr as *mut std::os::raw::c_void,
        );

        let p_starks_c12a = starks_new_c(
            p_config,
            self.config.c12a_const_pols.as_str(),
            self.config.map_const_pols_file,
            self.config.c12a_constants_tree.as_str(),
            self.config.c12a_stark_info.as_str(),
            proof_ctx.ptr as *mut std::os::raw::c_void,
        );

        let p_starks_recursive1 = starks_new_c(
            p_config,
            self.config.recursive1_const_pols.as_str(),
            self.config.map_const_pols_file,
            self.config.recursive1_constants_tree.as_str(),
            self.config.recursive1_stark_info.as_str(),
            proof_ctx.ptr as *mut std::os::raw::c_void,
        );

        let verkey_recursive2 = VerificationKey::<Goldilocks>::from_json(self.config.recursive2_verkey.as_str());

        timer_start!(SAVE_PUBLICS_JSON_BATCH_PROOF);

        let publics_stark_json: Vec<T> = proof_ctx.public_inputs.iter().cloned().take(proof_ctx.public_inputs.len() - 4).collect();

        timer_stop_and_log!(SAVE_PUBLICS_JSON_BATCH_PROOF);

        // Generate stark proof
        // ========================================================================
        let p_steps = zkevm_steps_new_c();
        let p_fri_proof = self.main_gen_proof(
            p_starks,
            self.config.zkevm_stark_info.as_str(),
            self.config.zkevm_verkey.as_str(),
            &proof_ctx.public_inputs,
            p_steps,
        );

        // Compute witness C12a
        // ========================================================================
        timer_start!(STARK_CALC_WITNESS_C12A);
        let stark_info_c12a = StarkInfo::from_json(self.config.c12a_stark_info.as_str());

        let roots_c = Vec::<Goldilocks>::new();
        let zkin = zkin_new_c(p_fri_proof, &publics_stark_json, &roots_c);

        let p_cm_pols12a = commit_pols_starks_new_c(
            proof_ctx.ptr as *mut std::os::raw::c_void,
            1 << stark_info_c12a.stark_struct.n_bits,
            stark_info_c12a.n_cm1,
        );

        circom_get_commited_pols_c(
            p_cm_pols12a,
            self.config.zkevm_verifier.as_str(),
            self.config.c12a_exec.as_str(),
            zkin,
            1 << stark_info_c12a.stark_struct.n_bits,
            stark_info_c12a.n_cm1,
        );

        commit_pols_starks_free_c(p_cm_pols12a);

        timer_stop_and_log!(STARK_CALC_WITNESS_C12A);

        // Generate C12a stark proof
        // ========================================================================
        let p_steps_c12a = c12a_steps_new_c();
        let p_fri_proof_c12a = self.main_gen_proof(
            p_starks_c12a,
            self.config.c12a_stark_info.as_str(),
            self.config.c12a_verkey.as_str(),
            &proof_ctx.public_inputs,
            p_steps_c12a,
        );

        // Compute witness recursive1
        // ========================================================================
        timer_start!(STARK_CALC_WITNESS_RECURSIVE1);
        let stark_info_recursive1 = StarkInfo::from_json(self.config.recursive1_stark_info.as_str());

        let mut roots_c_c12a = Vec::new();
        roots_c_c12a.push(verkey_recursive2.const_root[0]);
        roots_c_c12a.push(verkey_recursive2.const_root[1]);
        roots_c_c12a.push(verkey_recursive2.const_root[2]);
        roots_c_c12a.push(verkey_recursive2.const_root[3]);

        let zkin_c12a = zkin_new_c(p_fri_proof_c12a, &publics_stark_json, &roots_c_c12a);

        let p_cm_pols_recursive1 = commit_pols_starks_new_c(
            proof_ctx.ptr as *mut std::os::raw::c_void,
            1 << stark_info_recursive1.stark_struct.n_bits,
            stark_info_recursive1.n_cm1,
        );

        circom_recursive1_get_commited_pols_c(
            p_cm_pols_recursive1,
            self.config.recursive1_verifier.as_str(),
            self.config.recursive1_exec.as_str(),
            zkin_c12a,
            1 << stark_info_recursive1.stark_struct.n_bits,
            stark_info_recursive1.n_cm1,
        );

        commit_pols_starks_free_c(p_cm_pols_recursive1);

        timer_stop_and_log!(STARK_CALC_WITNESS_RECURSIVE1);

        // Generate recursive 1 stark proof
        // ========================================================================
        let p_steps_rec1 = recursive1_steps_new_c();
        let p_fri_proof_rec1 = self.main_gen_proof(
            p_starks_recursive1,
            self.config.recursive1_stark_info.as_str(),
            self.config.recursive1_verkey.as_str(),
            &proof_ctx.public_inputs,
            p_steps_rec1,
        );

        // Save proof
        // ========================================================================
        timer_start!(SAVE_PROOF);

        // let mut roots_c_recursive1 = Vec::<Goldilocks>::new();
        // let zkin_recursive1 = zkin_new_c(p_fri_proof_recursive1, &publics_stark_json, &roots_c_recursive1);

        // pProverRequest.batchProofOutput = zkinRecursive1;

        // save publics to filestarks
        // json2file(publicStarkJson, pProverRequest.publicsOutputFile());

        // Save output to file
        // if self.config.save_output_to_file {
            // json2file(pProverRequest.batchProofOutput, pProverRequest.filePrefix + "batch_proof.output.json");
        // }

        // Save proof to file
        // if self.config.save_proof_to_file {
            // jProofRecursive1["publics"] = publicStarkJson;
            // json2file(jProofRecursive1, pProverRequest.filePrefix + "batch_proof.proof.json");
        // }

        timer_stop_and_log!(SAVE_PROOF);
        config_free_c(p_config);

        fri_proof_free_c(p_fri_proof);
        fri_proof_free_c(p_fri_proof_c12a);
        fri_proof_free_c(p_fri_proof_rec1);

        zkevm_steps_free_c(p_steps);
        c12a_steps_free_c(p_steps_c12a);
        recursive1_steps_free_c(p_steps_rec1);

        starks_free_c(p_starks);
        starks_free_c(p_starks_c12a);
        starks_free_c(p_starks_recursive1);

        info!("{}: <-- eStark prover - STAGE {}", Self::MY_NAME, stage_id);
    }
}

impl<T> EStarkProver<T> {
    fn main_gen_proof(
        &self,
        p_starks: *mut ::std::os::raw::c_void,
        stark_info_filename: &str,
        verification_key_filename: &str,
        publics: &Vec<T>,
        p_steps: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void {
        timer_start!(STARKS_GENPROOF);

        let stark_info = StarkInfo::from_json(stark_info_filename);
        let p_fri_proof = self.generate_fri_proof(&stark_info);
        let verkey = VerificationKey::<Goldilocks>::from_json(verification_key_filename);

        starks_genproof_c::<T>(p_starks, p_fri_proof, publics, &verkey, p_steps);

        timer_stop_and_log!(STARKS_GENPROOF);

        p_fri_proof
    }

    fn generate_fri_proof(&self, stark_info: &StarkInfo) -> *mut ::std::os::raw::c_void {
        let pol_bits = stark_info.stark_struct.steps[stark_info.stark_struct.steps.len() - 1].n_bits;
        let num_trees = stark_info.stark_struct.steps.len() as u64;
        let eval_size = stark_info.ev_map.len() as u64;
        let n_publics = stark_info.n_publics;
        const FIELD_EXTENSION: u64 = 3;

        fri_proof_new_c(1 << pol_bits, FIELD_EXTENSION, num_trees, eval_size, n_publics)
    }
}
