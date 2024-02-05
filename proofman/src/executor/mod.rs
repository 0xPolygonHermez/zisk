pub mod executors_manager;
pub mod executors_manager_thread;
use crate::proof_ctx::ProofCtx;
use crate::config::Config;

// NOTE: config argument is added temporaly while integrating with zkevm-prover, remove when done
pub trait Executor<T> {
    fn witness_computation(&self, config: &dyn Config, stage_id: u32, proof_ctx: &mut ProofCtx<T>);
}

// pub trait ExecutorBase<T>: Sync {
//     fn get_name(&self) -> String;

//     fn _witness_computation(
//         &self,
//         config: &Box<dyn Config>,
//         stage_id: u32,
//         proof_ctx: Arc<RwLock<&mut ProofCtx<T>>>,
//         tasks: &TasksTable,
//         tx: SenderB<Message>,
//         rx: ReceiverB<Message>,
//     );

//     fn broadcast(&self, tx: &SenderB<Message>, payload: Payload) {
//         let msg = Message { src: self.get_name(), dst: "*".to_string(), payload };
//         tx.send(msg);
//     }
// }

#[macro_export]
macro_rules! executor {
    ($executor_name:ident: $base_element:ty) => {
        pub struct $executor_name {
            ptr: *mut u8,
        }

        unsafe impl Send for $executor_name {}
        unsafe impl Sync for $executor_name {}

        impl $executor_name {
            fn get_name(&self) -> String {
                stringify!($executor_name).to_string()
            }

            pub fn new() -> Self {
                $executor_name { ptr: std::ptr::null_mut() }
            }

            pub fn from_ptr(&self, ptr: *mut u8) -> Self {
                $executor_name { ptr }
            }
        }

        // impl $crate::executor::ExecutorBase<$base_element> for $executor_name {
        //     fn get_name(&self) -> String {
        //         stringify!($executor_name).to_string()
        //     }

        //     fn _witness_computation(
        //         &self,
        //         config: &Box<dyn $crate::config::Config>,
        //         stage_id: u32,
        //         proof_ctx: std::sync::Arc<std::sync::RwLock<&mut $crate::proof_ctx::ProofCtx<$base_element>>>,
        //         tasks: &$crate::task::TasksTable,
        //         tx: $crate::channel::SenderB<Message>,
        //         rx: $crate::channel::ReceiverB<Message>,
        //     ) {
        //         self.witness_computation(&config, stage_id, proof_ctx, tasks, &tx, &rx);
        //         self.broadcast(&tx, $crate::message::Payload::Finished);
        //         ()
        //     }
        // }
    };
}
