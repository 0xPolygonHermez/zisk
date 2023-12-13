use math::FieldElement;
use proofman::executor::Executor;
use proofman::proof_ctx::ProofCtx;
use proofman::message::Payload;
use proofman::channel::Channel;

use log::info;

pub struct ModuleExecutor<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> ModuleExecutor<T> {
    pub fn new() -> Self {
        ModuleExecutor {
            phantom: std::marker::PhantomData
        }
    }
}

impl<T: FieldElement> Executor<T> for ModuleExecutor<T> {
    fn witness_computation(&self, stage_id: u32, _subproof_id: i32, _instance_id: i32, _proof_ctx: &ProofCtx<T>, channel: Channel) {
        if stage_id != 1 {
            info!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let msg = channel.recv().unwrap();

        match msg.payload {
            Payload ::Halt => {
                println!("Halt!");
            },
            Payload::NewTrace { subproof_id, air_id } => {
                info!("ExModule> NewTrace: subproof_id: {}, air_id: {}", subproof_id, air_id);
            },
        }
    }
}