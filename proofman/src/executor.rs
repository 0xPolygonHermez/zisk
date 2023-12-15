use crate::proof_ctx::ProofCtx;
use crate::channel::{SenderB, ReceiverB};
use crate::message::{Message, Payload};

pub trait Executor<T>: Sync {
    fn witness_computation(&self, stage_id: u32, proof_ctx:&ProofCtx<T>, tx: SenderB<Message>, rx: ReceiverB<Message>);
}

pub trait ExecutorBase<T>: Sync {
    fn get_name(&self) -> String;
    
    fn _witness_computation(&self, stage_id: u32, proof_ctx:&ProofCtx<T>, tx: SenderB<Message>, rx: ReceiverB<Message>);

    fn broadcast(&self, tx: SenderB<Message>, payload: Payload) {
        let msg = Message {  
            src: self.get_name(),
            dst: "*".to_string(),
            payload
        };
        tx.send(msg);
    }
}

#[macro_export]
macro_rules! executor {
    ($executor_name:ident: $base_element:ty) => {
        pub struct $executor_name;

        impl $crate::executor::ExecutorBase<$base_element> for $executor_name {
            fn get_name(&self) -> String {
                stringify!($executor_name).to_string()
            }

            fn _witness_computation(&self, stage_id: u32, proof_ctx:&$crate::proof_ctx::ProofCtx<$base_element>, tx: $crate::channel::SenderB<Message>, rx: $crate::channel::ReceiverB<Message>) {
                // TODO change tx and pass &tx
                self.witness_computation(stage_id, proof_ctx, tx.clone(), rx);
                self.broadcast(tx, $crate::message::Payload::Finished);
                ()
            }
        }
    }    
}