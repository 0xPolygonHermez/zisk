use crate::proof_ctx::ProofCtx;
use std::sync::Arc;

pub trait Executor<T>: Sync {
    fn witness_computation(&self, stage_id: u32, subproof_id: u32, instance_id: i32, proof_ctx:&ProofCtx<T>);
}
