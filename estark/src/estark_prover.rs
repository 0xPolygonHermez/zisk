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
    pub const_pols_filename: String,
    pub map_const_pols_file: bool,
    pub const_tree_filename: String,
    pub stark_info_filename: String,
    pub verkey_filename: String,
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
        let stark_info = StarkInfo::from_json(&config.stark_info_filename);

        let verkey = VerificationKey::<Goldilocks>::from_json(&config.verkey_filename);

        let p_starks = starks_new_c(
            p_config,
            config.const_pols_filename.as_str(),
            config.map_const_pols_file,
            config.const_tree_filename.as_str(),
            config.stark_info_filename.as_str(),
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
