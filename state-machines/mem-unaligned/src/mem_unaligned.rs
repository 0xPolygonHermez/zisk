use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use sm_common::{MemUnalignedOp, OpResult, Provable, Sessionable, WorkerHandler, WorkerTask};
use wchelpers::WCComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct MemUnalignedSM {
    inputs: Mutex<Vec<MemUnalignedOp>>,
    worker_handlers: Vec<WorkerHandler<MemUnalignedOp>>,
}

#[allow(unused, unused_variables)]
impl MemUnalignedSM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let (tx, rx) = mpsc::channel();

        let worker_handle = Self::launch_thread(rx);

        let mem_aligned_sm = Self {
            inputs: Mutex::new(Vec::new()),
            worker_handlers: vec![WorkerHandler::new(tx, worker_handle)],
        };
        let mem_aligned_sm = Arc::new(mem_aligned_sm);

        wcm.register_component(mem_aligned_sm.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        mem_aligned_sm
    }

    fn read(
        &self,
        _addr: u64,
        _width: usize, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((0, true))
    }

    fn write(
        &self,
        _addr: u64,
        _width: usize,
        _val: u64, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((0, true))
    }

    fn launch_thread(rx: mpsc::Receiver<WorkerTask<MemUnalignedOp>>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            while let Ok(task) = rx.recv() {
                match task {
                    WorkerTask::Prove(inputs) => {
                        println!("Mem: Proving buffer");
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

impl<F> WCComponent<F> for MemUnalignedSM {
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

impl Provable<MemUnalignedOp, OpResult> for MemUnalignedSM {
    fn calculate(&self, operation: MemUnalignedOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            MemUnalignedOp::Read(addr, width) => self.read(addr, width),
            MemUnalignedOp::Write(addr, width, val) => self.write(addr, width, val),
        }
    }

    fn prove(&self, operations: &[MemUnalignedOp]) {
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
        operation: MemUnalignedOp,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation]);
        result
    }
}

impl Sessionable for MemUnalignedSM {
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
