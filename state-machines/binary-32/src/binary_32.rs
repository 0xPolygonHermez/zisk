use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

use std::{sync::mpsc, thread};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use sm_common::{Binary32Op, Provable, Sessionable, WorkerHandler, WorkerTask, ZiskResult};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Binary32SM {
    inputs: Arc<RwLock<Vec<Binary32Op>>>,
    worker_handler: Vec<WorkerHandler>,
    last_proved_idx: AtomicUsize,
}

impl Binary32SM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let inputs = RwLock::new(Vec::new());
        let (tx, rx) = mpsc::channel();

        let inputs = Arc::new(inputs);
        let inputs_clone = Arc::clone(&inputs);

        let worker_handle = Self::launch_thread(inputs_clone, rx);

        let binary32_sm = Self {
            inputs,
            worker_handler: vec![WorkerHandler::new(tx, worker_handle)],
            last_proved_idx: AtomicUsize::new(0),
        };
        let binary32_sm = Arc::new(binary32_sm);

        wcm.register_component(binary32_sm.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        binary32_sm
    }

    pub fn and(&self, a: u32, b: u32) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        Ok(((a & b) as u64, true))
    }

    fn launch_thread(
        inputs_clone: Arc<RwLock<Vec<Binary32Op>>>,
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
                                "Binary32SM: Proving [{:?}..[{:?}]",
                                inputs[low_idx],
                                inputs[high_idx - 1]
                            );
                        }
                        // thread::sleep(Duration::from_millis(1000));
                    }
                    WorkerTask::Finish => {
                        println!("Binary32SM: Task::Finish()");
                        break;
                    }
                };
            }
            println!("Binary32SM: Finishing the worker thread");
        })
    }
}

impl<F> WCComponent<F> for Binary32SM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Binary32Op, ZiskResult> for Binary32SM {
    fn calculate(&self, operation: Binary32Op) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        match operation {
            Binary32Op::And(a, b) => self.and(a, b),
        }
    }

    fn prove(&self, operations: &[Binary32Op]) {
        // Create a scoped block to hold the write lock only the necessary
        let num_inputs = {
            let mut inputs = self.inputs.write().unwrap();
            inputs.extend_from_slice(operations);
            inputs.len()
        };

        if num_inputs % PROVE_CHUNK_SIZE == 0 {
            let last_proved_idx = self.last_proved_idx.load(Ordering::Relaxed);
            println!("Binary32SM: Sending Task::Prove[{}..{}]", last_proved_idx, num_inputs);
            self.worker_handler[0].send(WorkerTask::Prove(last_proved_idx, num_inputs));
            self.last_proved_idx.store(num_inputs, Ordering::Relaxed);
        }
    }

    fn calculate_prove(
        &self,
        operation: Binary32Op,
    ) -> Result<ZiskResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for Binary32SM {
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
