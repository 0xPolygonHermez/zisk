use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

use std::{sync::mpsc, thread};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use sm_common::{Binary3264Op, Provable, Sessionable, WorkerHandler, WorkerTask, ZiskResult};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Binary3264SM {
    inputs: Arc<RwLock<Vec<Binary3264Op>>>,
    worker_handler: Vec<WorkerHandler>,
    last_proved_idx: AtomicUsize,
}

impl Binary3264SM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let inputs = RwLock::new(Vec::new());
        let (tx, rx) = mpsc::channel();

        let inputs = Arc::new(inputs);
        let inputs_clone = Arc::clone(&inputs);

        let worker_handle = Self::launch_thread(inputs_clone, rx);

        let binary3264_sm = Self {
            inputs,
            worker_handler: vec![WorkerHandler::new(tx, worker_handle)],
            last_proved_idx: AtomicUsize::new(0),
        };

        let binary3264_sm = Arc::new(binary3264_sm);

        wcm.register_component(binary3264_sm.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        binary3264_sm
    }

    pub fn and32(&self, a: u32, b: u32) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        Ok(((a & b) as u64, true))
    }

    pub fn and64(&self, a: u64, b: u64) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        Ok((a & b, true))
    }

    fn launch_thread(
        inputs_clone: Arc<RwLock<Vec<Binary3264Op>>>,
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
                                "Binary3264SM: Proving [{:?}..[{:?}]",
                                inputs[low_idx],
                                inputs[high_idx - 1]
                            );
                        }
                        // thread::sleep(Duration::from_millis(1000));
                    }
                    WorkerTask::Finish => {
                        println!("Binary3264SM: Task::Finish()");
                        break;
                    }
                };
            }
            println!("Binary3264SM: Finishing the worker thread");
        })
    }
}

impl<F> WCComponent<F> for Binary3264SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Binary3264Op, ZiskResult> for Binary3264SM {
    fn calculate(&self, operation: Binary3264Op) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        match operation {
            Binary3264Op::And32(a, b) => self.and32(a, b),
            Binary3264Op::And64(a, b) => self.and64(a, b),
        }
    }

    fn prove(&self, operations: &[Binary3264Op]) {
        // Create a scoped block to hold the write lock only the necessary
        let num_inputs = {
            let mut inputs = self.inputs.write().unwrap();
            inputs.extend_from_slice(operations);
            inputs.len()
        };

        if num_inputs % PROVE_CHUNK_SIZE == 0 {
            let last_proved_idx = self.last_proved_idx.load(Ordering::Relaxed);
            println!("Binary3264SM: Sending Task::Prove[{}..{}]", last_proved_idx, num_inputs);
            self.worker_handler[0].send(WorkerTask::Prove(last_proved_idx, num_inputs));
            self.last_proved_idx.store(num_inputs, Ordering::Relaxed);
        }
    }

    fn calculate_prove(
        &self,
        operation: Binary3264Op,
    ) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for Binary3264SM {
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
