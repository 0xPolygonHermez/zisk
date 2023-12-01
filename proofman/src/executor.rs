use crate::proof_ctx::ProofCtx;

use std::sync::{Arc, RwLock};

pub trait Executor: Send {
    fn witness_computation(&self, stage_id: u32, subproof_id: u32, instance_id: i32, proof_ctx: Arc<RwLock<ProofCtx>>, /*publics*/);
}
