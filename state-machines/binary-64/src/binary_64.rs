use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{sync::mpsc, thread};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use sm_common::{Binary64Op, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Binary64SM {
    inputs: Mutex<Vec<Binary64Op>>,
    worker_handlers: Vec<WorkerHandler<Binary64Op>>,
}

impl Binary64SM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let (tx, rx) = mpsc::channel();

        let worker_handle = Self::launch_thread(rx);

        let binary64_sm = Self {
            inputs: Mutex::new(Vec::new()),
            worker_handlers: vec![WorkerHandler::new(tx, worker_handle)],
        };
        let binary64_sm = Arc::new(binary64_sm);

        wcm.register_component(binary64_sm.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        binary64_sm
    }

    pub fn and(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a & b, true))
    }

    pub fn or(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a | b, true))
    }

    fn launch_thread(rx: mpsc::Receiver<WorkerTask<Binary64Op>>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            while let Ok(task) = rx.recv() {
                match task {
                    WorkerTask::Prove(inputs) => {
                        println!("Binary64SM: Proving buffer");
                        // thread::sleep(Duration::from_millis(1000));
                    }
                    WorkerTask::Finish => {
                        println!("Binary64SM: Task::Finish()");
                        break;
                    }
                };
            }
            println!("Binary64SM: Finishing the worker thread");
        })
    }
}

impl<F> WCComponent<F> for Binary64SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Binary64Op, OpResult> for Binary64SM {
    fn calculate(&self, operation: Binary64Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Binary64Op::And(a, b) => self.and(a, b),
            Binary64Op::Or(a, b) => self.or(a, b),
        }
    }

    fn prove(&self, operations: &[Binary64Op]) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);
            if inputs.len() >= PROVE_CHUNK_SIZE {
                let old_inputs = std::mem::take(&mut *inputs);
                self.worker_handlers[0].send(WorkerTask::Prove(Arc::new(old_inputs)));
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: Binary64Op,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for Binary64SM {
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
