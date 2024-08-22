use std::sync::{Arc, Mutex};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct Arith3264SM {
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

impl Arith3264SM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let arith3264_sm = Self { inputs: Mutex::new(Vec::new()) };
        let arith3264_sm = Arc::new(arith3264_sm);

        wcm.register_component(arith3264_sm.clone() as Arc<dyn WitnessComponent<F>>, Some(air_ids));

        arith3264_sm
    }
}

impl<F> WitnessComponent<F> for Arith3264SM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: usize,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for Arith3264SM {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredOperation], is_last: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);
            if is_last || inputs.len() >= PROVE_CHUNK_SIZE {
                let _inputs = std::mem::take(&mut *inputs);

                scope.spawn(move |_scope| {
                    // TODO! Implement prove _inputs (a chunk of operations)
                    println!("Arith3264: Finishing the worker thread");
                });
            }
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
