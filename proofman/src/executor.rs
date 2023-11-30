use crate::proof_ctx::ProofCtx;

use std::rc::Rc;
use std::cell::RefCell;

pub trait Executor {
    fn witness_computation(&self, stage_id: u32, subproof_id: u32, instance_id: i32, proof_ctx: Rc<RefCell<ProofCtx>>, /*publics*/);
}
