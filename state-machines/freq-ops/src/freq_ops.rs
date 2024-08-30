use std::sync::Mutex;

use proofman::WitnessComponent;
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{FreqOp, OpResult, Provable};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

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
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }

    fn register_predecessor(&self) {}

    fn unregister_predecessor(&self, _scope: &Scope) {}
}

impl Provable<FreqOp, OpResult> for FreqOpSM {
    fn calculate(&self, operation: FreqOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            FreqOp::Add(a, b) => self.add(a, b),
        }
    }

    fn prove(&self, operations: &[FreqOp], drain: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let _drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                scope.spawn(move |_| {
                    // TODO! Implement prove drained_inputs (a chunk of operations)
                });
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: FreqOp,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], drain, scope);
        result
    }
}
