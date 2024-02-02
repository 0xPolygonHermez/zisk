use crate::{executor::ExecutorBase, task::TasksTable};
use crate::channel::SenderB;
use crate::proof_ctx::ProofCtx;
use crate::message::Message;
use crate::channel::ReceiverB;
use crate::message::Payload;
use crate::config::Config;
use log::debug;
use std::sync::{Arc, RwLock};

// WITNESS CALCULATOR MANAGER
// ================================================================================================
pub struct WitnessCalculatorManager<T> {
    wc: Vec<Box<dyn ExecutorBase<T>>>,
    tasks: TasksTable,
}

impl<T: Clone + Send + Sync + std::fmt::Debug> WitnessCalculatorManager<T> {
    const MY_NAME: &'static str = "witnessm";

    pub fn new(wc: Vec<Box<dyn ExecutorBase<T>>>) -> Self {
        debug!("{}: Initializing...", Self::MY_NAME);

        WitnessCalculatorManager { wc, tasks: TasksTable::new() }
    }

    pub fn witness_computation(&self, stage_id: u32, config: &Box<dyn Config>, proof_ctx: &mut ProofCtx<T>) {
        debug!("{}: --> Computing witness stage {}", Self::MY_NAME, stage_id);

        let channel = SenderB::new();

        let arc_proof = Arc::new(RwLock::new(proof_ctx));

        std::thread::scope(|s| {
            for wc in self.wc.iter() {
                let tx = channel.clone();
                let rx = channel.subscribe();

                let proof_ctx_lock = Arc::clone(&arc_proof);

                // TODO! THIS IS A HACK TO AVOID STACK OVERFLOW DURING THE CALL TO ZKEVM PROVER BUT IT SHOULD BE FIXED
                // TODO! IN THE FUTURE OR SET A PARAMETER TO CONFIGURE THE STACK SIZE
                // We set the stack size to 4MB to avoid stack overflow during the call to zkevm prover
                std::thread::Builder::new()
                    .stack_size(8 * 1024 * 1024)
                    .spawn_scoped(s, move || {
                        wc._witness_computation(config, stage_id, proof_ctx_lock, &self.tasks, tx, rx);
                    })
                    .unwrap();
            }

            self.thread_manager(self.wc.len(), channel.clone(), channel.subscribe());
        });

        debug!("{}: <-- Computing witness stage {}", Self::MY_NAME, stage_id);
    }

    fn thread_manager(&self, num_threads: usize, _tx: SenderB<Message>, rx: ReceiverB<Message>) {
        let mut num_threads_finished = 0;
        loop {
            let msg = rx.recv().expect("Failed to receive message");

            if let Payload::Finished = msg.payload {
                num_threads_finished += 1;
                if num_threads_finished == num_threads {
                    debug!("{}: All threads finished", Self::MY_NAME);
                    break;
                }
            }
        }
    }
}
