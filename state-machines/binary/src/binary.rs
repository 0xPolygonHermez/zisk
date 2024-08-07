use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{fmt::Debug, sync::mpsc, thread};

use proofman::WitnessManager;
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_binary_32::Binary32SM;
use sm_binary_3264::Binary3264SM;
use sm_binary_64::Binary64SM;
use sm_common::{
    Binary3264Op, Binary32Op, Binary64Op, OpResult, Provable, Sessionable, Sessions, WorkerHandler,
    WorkerTask,
};
use witness_helpers::WitnessComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct BinarySM {
    inputs32: Mutex<Vec<Binary32Op>>,
    inputs64: Mutex<Vec<Binary64Op>>,
    binary32_sm: Arc<Binary32SM>,
    binary64_sm: Arc<Binary64SM>,
    binary3264_sm: Arc<Binary3264SM>,
}

impl BinarySM {
    pub fn new<F>(
        wcm: &mut WitnessManager<F>,
        binary32_sm: Arc<Binary32SM>,
        binary64_sm: Arc<Binary64SM>,
        binary3264_sm: Arc<Binary3264SM>,
    ) -> Arc<Self> {
        let binary_sm = Self {
            inputs32: Mutex::new(Vec::new()),
            inputs64: Mutex::new(Vec::new()),
            binary32_sm,
            binary64_sm,
            binary3264_sm,
        };
        let binary_sm = Arc::new(binary_sm);

        wcm.register_component(binary_sm.clone() as Arc<dyn WitnessComponent<F>>, None);

        binary_sm
    }
}

impl<F> WitnessComponent<F> for BinarySM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<Binary3264Op, OpResult> for BinarySM {
    fn calculate(&self, operation: Binary3264Op) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            Binary3264Op::And32(a, b) => self.binary32_sm.and(a, b),
            Binary3264Op::And64(a, b) => self.binary64_sm.and(a, b),
            Binary3264Op::Or32(a, b) => self.binary32_sm.or(a, b),
            Binary3264Op::Or64(a, b) => self.binary64_sm.or(a, b),
        }
    }

    fn prove(&self, operations: &[Binary3264Op], is_last: bool, scope: &Scope) {
        let mut inputs32 = self.inputs32.lock().unwrap();
        let mut inputs64 = self.inputs64.lock().unwrap();

        // TODO Split the operations into 32 and 64 bit operations in parallel
        for operation in operations {
            match operation {
                Binary3264Op::And32(a, b) | Binary3264Op::And32(a, b) => {
                    inputs32.push(operation.clone().into());
                }
                Binary3264Op::And64(a, b) | Binary3264Op::And64(a, b) => {
                    inputs64.push(operation.clone().into());
                }
                Binary3264Op::Or32(a, b) | Binary3264Op::Or32(a, b) => {
                    inputs32.push(operation.clone().into());
                }
                Binary3264Op::Or64(a, b) | Binary3264Op::Or64(a, b) => {
                    inputs64.push(operation.clone().into());
                }
            }
        }

        // The following is a way to release the lock on the inputs32 and inputs64 Mutexes asap
        // NOTE: The `inputs32` lock is released when it goes out of scope because it is shadowed
        let inputs32 = if is_last || inputs32.len() >= PROVE_CHUNK_SIZE {
            let _inputs32 = std::mem::take(&mut *inputs32);
            if _inputs32.is_empty() {
                None
            } else {
                Some(_inputs32)
            }
        } else {
            None
        };

        // NOTE: The `inputs64` lock is released when it goes out of scope because it is shadowed
        let inputs64 = if is_last || inputs64.len() >= PROVE_CHUNK_SIZE {
            let _inputs64 = std::mem::take(&mut *inputs64);
            if _inputs64.is_empty() {
                None
            } else {
                Some(_inputs64)
            }
        } else {
            None
        };

        if inputs32.is_some() {
            let binary32_s = self.binary32_sm.clone();
            scope.spawn(move |scope| {
                binary32_s.prove(&inputs32.unwrap(), is_last, scope);
            });
        }

        if inputs64.is_some() {
            let binary64_sm = self.binary64_sm.clone();
            scope.spawn(move |scope| {
                binary64_sm.prove(&inputs64.unwrap(), is_last, scope);
            });
        }
    }

    fn calculate_prove(
        &self,
        operation: Binary3264Op,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
