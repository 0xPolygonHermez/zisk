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
    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<F>) -> ProverStatus;
    fn opening_stage(
        &mut self,
        opening_id: u32,
        proof_ctx: &mut ProofCtx<F>,
        transcript: &mut FFITranscript,
    ) -> ProverStatus;

    // fn get_subproof_values<T>(&self) -> Vec<T>;

    fn get_map_offsets(&self, stage: &str, is_extended: bool) -> u64;
    fn add_challenges_to_transcript(&self, stage: u64, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript);
    fn add_publics_to_transcript(&self, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript);

    fn verify_constraints(&self, stage: u32) -> bool;
}
