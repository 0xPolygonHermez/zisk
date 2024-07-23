use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

use std::{sync::mpsc, thread};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use sm_common::{FreqOp, Provable, Sessionable, WorkerHandler, WorkerTask, ZiskResult};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct FreqOpSM {
    inputs: Arc<RwLock<Vec<FreqOp>>>,
    worker_handler: Vec<WorkerHandler>,
    last_proved_idx: AtomicUsize,
}

impl Default for FreqOpSM {
    fn default() -> Self {
        Self::new()
    }
}

impl FreqOpSM {
    pub fn new() -> Self {
        let inputs = RwLock::new(Vec::new());
        let (tx, rx) = mpsc::channel();

        let inputs = Arc::new(inputs);
        let inputs_clone = Arc::clone(&inputs);

        let worker_handle = Self::launch_thread(inputs_clone, rx);

        Self {
            inputs,
            worker_handler: vec![WorkerHandler::new(tx, worker_handle)],
            last_proved_idx: AtomicUsize::new(0),
        }
    }

    fn add(&self, a: u64, b: u64) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        Ok((a + b, true))
    }

    fn launch_thread(
        inputs_clone: Arc<RwLock<Vec<FreqOp>>>,
        rx: mpsc::Receiver<WorkerTask>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let inputs = inputs_clone;
            while let Ok(task) = rx.recv() {
                match task {
                    WorkerTask::Prove(low_idx, high_idx) => {
                        {
                            let inputs = inputs.read().unwrap();
                            println!(
                                "FreqOpSM: Proving [{:?}..[{:?}]",
                                inputs[low_idx],
                                inputs[high_idx - 1]
                            );
                        }
                        // thread::sleep(Duration::from_millis(1000));
                    }
                    WorkerTask::Finish => {
                        println!("FreqOpSM: Task::Finish()");
                        break;
                    }
                };
            }
            println!("FreqOpSM: Finishing the worker thread");
        })
    }
}

impl<F> WCComponent<F> for FreqOpSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: &AirInstance,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<FreqOp, ZiskResult> for FreqOpSM {
    fn calculate(&self, operation: FreqOp) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        match operation {
            FreqOp::Add(a, b) => self.add(a, b),
        }
    }

    fn prove(&self, operations: &[FreqOp]) {
        // Create a scoped block to hold the write lock only the necessary
        let num_inputs = {
            let mut inputs = self.inputs.write().unwrap();
            inputs.extend_from_slice(operations);
            inputs.len()
        };

        if num_inputs % PROVE_CHUNK_SIZE == 0 {
            let last_proved_idx = self.last_proved_idx.load(Ordering::Relaxed);
            println!("FreqOpSM: Sending Task::Prove[{}..{}]", last_proved_idx, num_inputs);
            self.worker_handler[0].send(WorkerTask::Prove(last_proved_idx, num_inputs));
            self.last_proved_idx.store(num_inputs, Ordering::Relaxed);
        }
    }

    fn calculate_prove(&self, operation: FreqOp) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for FreqOpSM {
    fn when_closed(&self) {
        let num_inputs = { self.inputs.read().unwrap().len() };

        let last_proved_idx = self.last_proved_idx.load(Ordering::Relaxed);
        if num_inputs - last_proved_idx > 0 {
            self.worker_handler[0].send(WorkerTask::Prove(last_proved_idx, num_inputs));
        }

        for worker in &self.worker_handler {
            worker.send(WorkerTask::Finish);
            worker.terminate();
        }
    }
}
