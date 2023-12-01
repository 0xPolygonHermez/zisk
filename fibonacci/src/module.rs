use log::debug;

use proofman::executor::Executor;

use std::sync::{Arc, RwLock};

use proofman::proof_ctx::ProofCtx;

pub struct Module {
    name: String
}

impl Module {
    pub fn new() -> Self {
        Module { name: "Module    ".to_string() }
    }
}

impl Executor for Module {
    fn witness_computation(&self, stage_id: u32, _subproof_id: u32, _instance_id: i32, _proof_ctx: Arc<RwLock<ProofCtx>>/*, publics*/) {
        debug!("[{}] > Witness computation for stage {}", self.name, stage_id);
        println!("!!!!!!!");
    }
}
