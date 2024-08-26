use std::{mem, sync::Mutex};

use proofman::WitnessComponent;
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{FreqOp, OpResult, Provable};

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct FreqOpSM {
    inputs: Mutex<Vec<FreqOp>>,
}

impl Default for FreqOpSM {
    fn default() -> Self {
        Self::new()
    }
}

impl FreqOpSM {
    pub fn new() -> Self {
        Self { inputs: Mutex::new(Vec::new()) }
    }

    fn add(&self, a: u64, b: u64) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((a + b, true))
    }
}

impl<F> WitnessComponent<F> for FreqOpSM {
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

impl Provable<FreqOp, OpResult> for FreqOpSM {
    fn calculate(&self, operation: FreqOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            FreqOp::Add(a, b) => self.add(a, b),
        }
    }

    fn prove(&self, operations: &[FreqOp], is_last: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);
            if is_last || inputs.len() >= PROVE_CHUNK_SIZE {
                let _inputs = mem::take(&mut *inputs);

                scope.spawn(move |_scope| {
                    // TODO! Implement prove _inputs (a chunk of operations)
                });
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: FreqOp,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
