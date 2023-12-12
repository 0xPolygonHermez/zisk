use crate::proof_ctx::ProofCtx;
use crate::message::Message;
use crossbeam_channel::{Receiver, Sender};


pub trait Executor<T>: Sync {
    fn get_name(&self) -> &str;
    fn witness_computation(&self, stage_id: u32, subproof_id: u32, instance_id: i32, proof_ctx:&ProofCtx<T>, tx: Sender<Message>, rx: Receiver<Message>);
}
