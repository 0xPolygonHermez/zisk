use crate::proof_ctx::ProofCtx;
use std::sync::Arc;

pub trait Executor<T> {
    fn witness_computation(&self, stage_id: u32, subproof_id: u32, instance_id: i32, proof_ctx: Arc<ProofCtx<T>>);
}
