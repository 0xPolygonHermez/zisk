use crate::{executor::ExecutorBase, task::TasksTable};
use crate::channel::SenderB;
use crate::proof_ctx::ProofCtx;
use crate::message::Message;
use crate::channel::ReceiverB;
use crate::message::Payload;
use log::{debug, info};

// WITNESS CALCULATOR MANAGER
// ================================================================================================
pub struct WitnessCalculatorManager<T> {
    wc: Vec<Box<dyn ExecutorBase<T>>>,
    tasks: TasksTable
}

impl<T: Clone + Send + Sync + std::fmt::Debug> WitnessCalculatorManager<T> {
    const MY_NAME: &'static str = "witnessm";

    pub fn new(wc: Vec<Box<dyn ExecutorBase<T>>>) -> Self {
        debug!("{}> Initializing...", Self::MY_NAME);

        WitnessCalculatorManager {
            wc,
            tasks: TasksTable::new()
        }
    }

    pub fn witness_computation(&self, stage_id: u32, proof_ctx: &ProofCtx<T>) {
        debug!("{}> Computing witness stage {}", Self::MY_NAME, stage_id);
        
        let channel = SenderB::new();

        std::thread::scope(|s| {
            for wc in self.wc.iter() {
                let tx = channel.clone();
                let rx = channel.subscribe();
                s.spawn(move || {
                    wc._witness_computation(stage_id, proof_ctx, &self.tasks, tx, rx);
                });
            }

            self.thread_manager(self.wc.len(), channel.clone(), channel.subscribe());
        });

        debug!("[{}] > Computing witness stage {}", Self::MY_NAME, stage_id);
    }

    fn thread_manager(&self, num_threads: usize, _tx: SenderB<Message>, rx: ReceiverB<Message>) {
        let mut num_threads_finished = 0;
        loop {
            let msg = rx.recv().expect("Failed to receive message");

            if let Payload::Finished = msg.payload {
                num_threads_finished += 1;
                if num_threads_finished == num_threads {
                    info!("All threads finished");
                    break;
                }
            }
        }
    }
}
