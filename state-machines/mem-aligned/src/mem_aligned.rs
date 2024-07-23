use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, RwLock,
    },
    thread,
};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use sm_common::{MemOp, MemOpResult, Provable, Sessionable, WorkerHandler, WorkerTask};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct MemAlignedSM {
    inputs: Arc<RwLock<Vec<MemOp>>>,
    worker_handler: Vec<WorkerHandler>,
    last_proved_idx: AtomicUsize,
}

#[allow(unused, unused_variables)]
impl MemAlignedSM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let inputs = RwLock::new(Vec::new());
        let (tx, rx) = mpsc::channel();

        let inputs = Arc::new(inputs);
        let inputs_clone = Arc::clone(&inputs);

        let worker_handle = Self::launch_thread(inputs_clone, rx);

        let mem_aligned_sm = Self {
            inputs,
            worker_handler: vec![WorkerHandler::new(tx, worker_handle)],
            last_proved_idx: AtomicUsize::new(0),
        };
        let mem_aligned_sm = Arc::new(mem_aligned_sm);

        wcm.register_component(mem_aligned_sm.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        mem_aligned_sm
    }

    fn read(
        &self,
        _addr: u64, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<MemOpResult, Box<dyn std::error::Error>> {
        Ok(MemOpResult::Read(0))
    }

    fn write(
        &self,
        _addr: u64,
        _val: u64, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<MemOpResult, Box<dyn std::error::Error>> {
        Ok(MemOpResult::Write)
    }

    fn launch_thread(
        inputs_clone: Arc<RwLock<Vec<MemOp>>>,
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
                                "Mem: Proving [{:?}..[{:?}]",
                                inputs[low_idx],
                                inputs[high_idx - 1]
                            );
                        }
                        // thread::sleep(Duration::from_millis(1000));
                    }
                    WorkerTask::Finish => {
                        println!("Mem: Task::Finish()");
                        break;
                    }
                };
            }
            println!("Arith32SM: Finishing the worker thread");
        })
    }
}

impl<F> WCComponent<F> for MemAlignedSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: &AirInstance,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }

    fn suggest_plan(&self, _ectx: &mut ExecutionCtx) {}
}

impl Provable<MemOp, MemOpResult> for MemAlignedSM {
    fn calculate(&self, operation: MemOp) -> Result<MemOpResult, Box<dyn std::error::Error>> {
        match operation {
            MemOp::Read(addr) => self.read(addr),
            MemOp::Write(addr, val) => self.write(addr, val),
        }
    }

    fn prove(&self, operations: &[MemOp]) {
        // Create a scoped block to hold the write lock only the necessary
        let num_inputs = {
            let mut inputs = self.inputs.write().unwrap();
            inputs.extend_from_slice(operations);
            inputs.len()
        };

        if num_inputs % PROVE_CHUNK_SIZE == 0 {
            let last_proved_idx = self.last_proved_idx.load(Ordering::Relaxed);
            println!("Mem: Sending Task::Prove[{}..{}]", last_proved_idx, num_inputs);
            self.worker_handler[0].send(WorkerTask::Prove(last_proved_idx, num_inputs));
            self.last_proved_idx.store(num_inputs, Ordering::Relaxed);
        }
    }

    fn calculate_prove(&self, operation: MemOp) -> Result<MemOpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for MemAlignedSM {
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
