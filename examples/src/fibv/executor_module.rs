use math::FieldElement;
use proofman::executor::Executor;
use proofman::proof_ctx::ProofCtx;
use crossbeam_channel::{Receiver, Sender};
use proofman::message::{Message, Payload};
use log::info;

use log::debug;

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
    fn witness_computation(&self, stage_id: u32, _subproof_id: i32, _instance_id: i32, _proof_ctx: &ProofCtx<T>, _tx: Sender<Message>, rx: Receiver<Message>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let msg = rx.recv().unwrap();

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