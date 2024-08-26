use std::sync::{Arc, Mutex};

use crate::{Arith3264SM, Arith32SM, Arith64SM};
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 3;

#[allow(dead_code)]
pub struct ArithSM {
    inputs32: Mutex<Vec<ZiskRequiredOperation>>,
    inputs64: Mutex<Vec<ZiskRequiredOperation>>,
    arith32_sm: Arc<Arith32SM>,
    arith64_sm: Arc<Arith64SM>,
    arith3264_sm: Arc<Arith3264SM>,
}

impl ArithSM {
    pub fn new<F>(
        wcm: &mut WitnessManager<F>,
        arith32_sm: Arc<Arith32SM>,
        arith64_sm: Arc<Arith64SM>,
        arith3264_sm: Arc<Arith3264SM>,
    ) -> Arc<Self> {
        let arith_sm = Self {
            inputs32: Mutex::new(Vec::new()),
            inputs64: Mutex::new(Vec::new()),
            arith32_sm,
            arith64_sm,
            arith3264_sm,
        };
        let arith_sm = Arc::new(arith_sm);

        wcm.register_component(arith_sm.clone() as Arc<dyn WitnessComponent<F>>, None);

        arith_sm
    }
}

impl<F> WitnessComponent<F> for ArithSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: usize,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for ArithSM {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredOperation], is_last: bool, scope: &Scope) {
        let mut _inputs32 = Vec::new();
        let mut _inputs64 = Vec::new();

        let operations32 = Arith32SM::operations();
        let operations64 = Arith64SM::operations();

        // TODO Split the operations into 32 and 64 bit operations in parallel
        for operation in operations {
            if operations32.contains(&operation.opcode) {
                _inputs32.push(operation.clone());
            }
            if operations64.contains(&operation.opcode) {
                _inputs64.push(operation.clone());
            } else {
                panic!("ArithSM: Operator {:x} not found", operation.opcode);
            }
        }

        let mut inputs32 = self.inputs32.lock().unwrap();
        let mut inputs64 = self.inputs64.lock().unwrap();

        inputs32.extend(_inputs32);
        inputs64.extend(_inputs64);

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
            let arith32_s = self.arith32_sm.clone();
            scope.spawn(move |scope| {
                arith32_s.prove(&inputs32.unwrap(), is_last, scope);
            });
        }

        if inputs64.is_some() {
            let arith64_sm = self.arith64_sm.clone();
            scope.spawn(move |scope| {
                arith64_sm.prove(&inputs64.unwrap(), is_last, scope);
            });
        }
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredOperation,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
