use std::os::raw::c_void;
use std::os::raw::c_char;
use std::sync::Arc;

use transcript::FFITranscript;

use crate::ProofCtx;

#[derive(Debug, PartialEq)]
pub enum ProverStatus {
    CommitStage,
    OpeningStage,
    StagesCompleted,
}
#[derive(Clone, PartialEq)]
pub enum ProofType {
    Basic,
    Compressor,
    Recursive1,
    Recursive2,
    Final,
}

pub struct ProverInfo {
    pub airgroup_id: usize,
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
    fn build(&mut self, proof_ctx: Arc<ProofCtx<F>>);
    fn new_transcript(&self) -> FFITranscript;
    fn num_stages(&self) -> u32;
    fn num_opening_stages(&self) -> u32;
    fn get_challenges(&self, stage_id: u32, proof_ctx: Arc<ProofCtx<F>>, transcript: &FFITranscript);
    fn calculate_stage(&mut self, stage_id: u32, proof_ctx: Arc<ProofCtx<F>>);
    fn commit_stage(&mut self, stage_id: u32, proof_ctx: Arc<ProofCtx<F>>) -> ProverStatus;
    fn calculate_xdivxsub(&mut self, proof_ctx: Arc<ProofCtx<F>>);
    fn calculate_lev(&mut self, proof_ctx: Arc<ProofCtx<F>>);
    fn opening_stage(&mut self, opening_id: u32, proof_ctx: Arc<ProofCtx<F>>) -> ProverStatus;

    fn get_buff_helper_size(&self) -> usize;
    fn get_proof(&self) -> *mut c_void;
    fn get_prover_info(&self) -> ProverInfo;
    fn save_proof(&self, proof_ctx: Arc<ProofCtx<F>>, output_dir: &str, save_json: bool) -> *mut c_void;

    fn get_transcript_values(&self, stage: u64, proof_ctx: Arc<ProofCtx<F>>) -> Option<Vec<F>>;
    fn calculate_hash(&self, values: Vec<F>) -> Vec<F>;
    fn verify_constraints(&self, proof_ctx: Arc<ProofCtx<F>>) -> Vec<ConstraintInfo>;
}
