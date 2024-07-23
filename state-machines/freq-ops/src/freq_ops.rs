use std::sync::{Arc, Mutex};

use std::{sync::mpsc, thread};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use sm_common::{FreqOp, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct FreqOpSM {
    inputs: Mutex<Vec<FreqOp>>,
    worker_handlers: Vec<WorkerHandler<FreqOp>>,
}

impl Default for FreqOpSM {
    fn default() -> Self {
        Self::new()
    }
}

impl FreqOpSM {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let worker_handle = Self::launch_thread(rx);

        Self {
            inputs: Mutex::new(Vec::new()),
            worker_handlers: vec![WorkerHandler::new(tx, worker_handle)],
        }
    }

    fn add(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a + b, true))
    }

    fn launch_thread(rx: mpsc::Receiver<WorkerTask<FreqOp>>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            while let Ok(task) = rx.recv() {
                match task {
                    WorkerTask::Prove(_inputs) => {
                        println!("Arith32SM: Proving buffer");
                        // thread::sleep(Duration::from_millis(1000));
                    }
                    WorkerTask::Finish => {
                        println!("Arith32SM: Task::Finish()");
                        break;
                    }
                };
            }
            println!("Arith32SM: Finishing the worker thread");
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

impl Provable<FreqOp, OpResult> for FreqOpSM {
    fn calculate(&self, operation: FreqOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            FreqOp::Add(a, b) => self.add(a, b),
        }
    }

    fn prove(&self, operations: &[FreqOp]) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);
            if inputs.len() >= PROVE_CHUNK_SIZE {
                let old_inputs = std::mem::take(&mut *inputs);
                self.worker_handlers[0].send(WorkerTask::Prove(Arc::new(old_inputs)));
            }
        }
    }

    fn calculate_prove(&self, operation: FreqOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for FreqOpSM {
    fn when_closed(&self) {
        if let Ok(mut inputs) = self.inputs.lock() {
            if !inputs.is_empty() {
                let old_inputs = std::mem::take(&mut *inputs);
                self.worker_handlers[0].send(WorkerTask::Prove(Arc::new(old_inputs)));
            }
        }

        for worker in &self.worker_handlers {
            worker.send(WorkerTask::Finish);
            worker.terminate();
        }
    }
}
