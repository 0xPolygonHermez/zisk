use goldilocks::Goldilocks;
use proofman::provers_manager::Prover;
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
    p_starks: *mut ::std::os::raw::c_void,
    p_steps: *mut ::std::os::raw::c_void,
    stark_info: StarkInfo,
    verkey: VerificationKey<Goldilocks>,
    phantom: std::marker::PhantomData<T>,
}

impl<T> EStarkProver<T> {
    const MY_NAME: &'static str = "estark  ";

    pub fn new(
        config: &EStarkProverConfig,
        p_steps: *mut std::os::raw::c_void,
        ptr: *mut std::os::raw::c_void,
    ) -> Self {
        timer_start!(ESTARK_PROVER_NEW);

        let p_config = config_new_c(&config.current_path);
        let stark_info = StarkInfo::from_json(&config.zkevm_stark_info);

        let verkey = VerificationKey::<Goldilocks>::from_json(&config.zkevm_verkey);

        let p_starks = starks_new_c(
            p_config,
            config.zkevm_const_pols.as_str(),
            config.map_const_pols_file,
            config.zkevm_constants_tree.as_str(),
            config.zkevm_stark_info.as_str(),
            ptr,
        );

        timer_stop_and_log!(ESTARK_PROVER_NEW);

        Self { p_starks, p_steps, stark_info, verkey, phantom: std::marker::PhantomData }
    }
}

impl<T> Prover<T> for EStarkProver<T> {
    fn compute_stage(&self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) {
        info!("{}: --> eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        timer_start!(STARKS_GENPROOF);

        let n_bits = self.stark_info.stark_struct.steps[self.stark_info.stark_struct.steps.len() - 1].n_bits;
        let n_trees = self.stark_info.stark_struct.steps.len() as u64;
        let n_publics = self.stark_info.n_publics;
        let eval_size = self.stark_info.ev_map.len() as u64;
        const FIELD_EXTENSION: u64 = 3;

        let p_fri_proof = fri_proof_new_c(1 << n_bits, FIELD_EXTENSION, n_trees, eval_size, n_publics);

        starks_genproof_c::<T>(self.p_starks, p_fri_proof, &proof_ctx.public_inputs, &self.verkey, self.p_steps);

        timer_stop_and_log!(STARKS_GENPROOF);

        proof_ctx.proof = Some(p_fri_proof);
        info!("{}: <-- eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        return;
    }
}
