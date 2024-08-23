use std::sync::{Arc, Mutex};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 7;

pub struct QuickOpsSM {
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

impl QuickOpsSM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let quickops_sm = Self { inputs: Mutex::new(Vec::new()) };
        let quickops_sm = Arc::new(quickops_sm);

        wcm.register_component(quickops_sm.clone() as Arc<dyn WitnessComponent<F>>, Some(air_ids));

        quickops_sm
    }

    pub fn operations() -> Vec<u8> {
        vec![0xb6, 0xb7, 0xbe, 0xbf]
    }
}

impl<F> WitnessComponent<F> for QuickOpsSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: usize,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for QuickOpsSM {
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
                    println!("QuickOps: Finishing the worker thread");
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
