use std::os::raw::c_void;
use std::os::raw::c_char;

use transcript::FFITranscript;

use crate::ProofCtx;

#[derive(Debug, PartialEq)]
pub enum ProverStatus {
    CommitStage,
    OpeningStage,
    StagesCompleted,
}

pub struct ProverInfo {
    pub air_group_id: usize,
    pub air_id: usize,
    pub prover_idx: usize,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ConstraintRowInfo {
   pub row: u64,
   pub dim: u64,
   pub value: [u64; 3usize],
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ConstraintInfo {
   pub id: u64,
   pub stage: u64,
   pub im_pol: bool,
   pub line: *const c_char,
   pub n_rows: u64,
   pub rows: [ConstraintRowInfo; 10usize],
}

#[derive(Debug, Clone, Copy)]

#[repr(C)]
pub struct ConstraintsResults {
    pub n_constraints: u64,
    pub constraints_info: *mut ConstraintInfo,
}

pub trait Prover<F> {
    fn build(&mut self, proof_ctx: &mut ProofCtx<F>);
    fn new_transcript(&self) -> FFITranscript;
    fn num_stages(&self) -> u32;
    fn num_opening_stages(&self) -> u32;
    fn get_challenges(&self, stage_id: u32, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript);
    fn calculate_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<F>);
    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<F>) -> ProverStatus;
    fn opening_stage(
        &mut self,
        opening_id: u32,
        proof_ctx: &mut ProofCtx<F>,
        transcript: &mut FFITranscript,
    ) -> ProverStatus;

    fn get_proof(&self) -> *mut c_void;
    fn get_prover_info(&self) -> ProverInfo;
    fn save_proof(&self, id: u64, output_dir: &str);

    fn add_challenges_to_transcript(&self, stage: u64, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript);
    fn add_publics_to_transcript(&self, proof_ctx: &mut ProofCtx<F>, transcript: &FFITranscript);

    fn verify_constraints(&self, proof_ctx: &mut ProofCtx<F>) -> Vec<ConstraintInfo>;
}
