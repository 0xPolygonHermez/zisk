use crate::proof_ctx::ProofCtx;
use crate::channel::{SenderB, ReceiverB};
use crate::message::Message;

pub trait Executor<T>: Sync {
    fn witness_computation(&self, stage_id: u32, proof_ctx:&ProofCtx<T>, tx: SenderB<Message>, rx: ReceiverB<Message>);
}
