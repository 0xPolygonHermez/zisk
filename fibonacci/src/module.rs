use log::{debug};

use proofman::executor::Executor;

pub struct Module {
    name: String
}

impl Module {
    pub fn new() -> Self {
        Module { name: "Module    ".to_string() }
    }
}

impl Executor for Module {
    fn witness_computation(&self, stage_id: u32, subproof_id: u32, instance_id: i32/*, publics*/) {
        debug!("[{}] > Witness computation for stage {}", self.name, stage_id);
    }
}
