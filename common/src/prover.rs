use std::os::raw::c_void;

use transcript::FFITranscript;

use crate::ProofCtx;

#[derive(Debug, PartialEq)]
pub enum ProverStatus {
    CommitStage,
    OpeningStage,
    StagesCompleted,
}

pub trait Prover<F> {
    fn build(&mut self, proof_ctx: &mut ProofCtx<F>, air_idx: usize);
    fn new_transcript(&self) -> FFITranscript;
    fn num_stages(&self) -> u32;
    fn num_opening_stages(&self) -> u32;
    fn get_challenges(&self, stage_id: u32, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript);
    fn calculate_stage(&mut self, stage_id: u32, pctx: &mut ProofCtx<F>);
    fn commit_stage(&mut self, stage_id: u32) -> ProverStatus;
    fn opening_stage(
        &mut self,
        opening_id: u32,
        proof_ctx: &mut ProofCtx<F>,
        transcript: &mut FFITranscript,
    ) -> ProverStatus;

    fn get_proof(&self) -> *mut c_void;
    fn save_proof(&self, id: u64, output_dir: &str);

    fn add_challenges_to_transcript(&self, stage: u64, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript);
    fn add_publics_to_transcript(&self, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript);

    fn verify_constraints(&self, stage: u32) -> bool;
}
