use std::sync::{Arc, Mutex};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Arith32SM {
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

impl Arith32SM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let arith32_sm = Self { inputs: Mutex::new(Vec::new()) };
        let arith32_sm = Arc::new(arith32_sm);

        wcm.register_component(arith32_sm.clone() as Arc<dyn WitnessComponent<F>>, Some(air_ids));

        arith32_sm
    }

    pub fn operations() -> Vec<u8> {
        vec![0xb6, 0xb7, 0xbe, 0xbf]
    }
}

impl<F> WitnessComponent<F> for Arith32SM {
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

impl Provable<ZiskRequiredOperation, OpResult> for Arith32SM {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                let _drained_inputs = inputs.drain(..PROVE_CHUNK_SIZE).collect::<Vec<_>>();

                scope.spawn(move |_| {
                    // TODO! Implement prove drained_inputs (a chunk of operations)
                });
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredOperation,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], drain, scope);
        result
    }
}
