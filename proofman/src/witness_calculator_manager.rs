use crate::executor::ExecutorBase;
use crate::channel::SenderB;
use crate::proof_ctx::ProofCtx;
use crate::message::Message;
use crate::channel::ReceiverB;
use crate::message::Payload;
use log::{debug, info};

// WITNESS CALCULATOR MANAGER
// ================================================================================================
pub struct WitnessCalculatorManager<T> {
    wc: Vec<Box<dyn ExecutorBase<T>>>
}

impl<T: Clone + Send + Sync + std::fmt::Debug> WitnessCalculatorManager<T> {
    const MY_NAME: &'static str = "witnessm";

    pub fn new(wc: Vec<Box<dyn ExecutorBase<T>>>) -> Self {
        debug!("{}> Initializing...", Self::MY_NAME);

        WitnessCalculatorManager {
            wc
        }
    }

    pub fn witness_computation(&self, stage_id: u32, proof_ctx: &ProofCtx<T>) {
        debug!("{}> Computing witness stage {}", Self::MY_NAME, stage_id);

        // TODO create a channel constructor and use it here. Add Clone trait to clone the channel for each wc
        
        let channel = SenderB::new();

        std::thread::scope(|s| {
            for wc in self.wc.iter() {
                let tx = channel.clone();
                let rx = channel.subscribe();
                s.spawn(move || {
                    wc._witness_computation(stage_id, proof_ctx, tx, rx);
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

            if msg.payload == Payload::Finished {
                num_threads_finished += 1;
                if num_threads_finished == num_threads {
                    info!("All threads finished");
                    break;
                }
            }
        }
    }
}
