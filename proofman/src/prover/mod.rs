pub mod provers_manager;

use crate::proof_ctx::ProofCtx;

pub trait Prover<T> {
    fn compute_stage(&self, stage_id: u32, proof_ctx: &mut ProofCtx<T>);
}
