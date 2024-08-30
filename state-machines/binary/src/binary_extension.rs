use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct BinaryExtensionSM {
    registered_predecessors: AtomicU32,
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

impl BinaryExtensionSM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let binary_extension_sm =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let binary_extension_sm = Arc::new(binary_extension_sm);

        wcm.register_component(binary_extension_sm.clone(), Some(air_ids));

        binary_extension_sm
    }

    pub fn operations() -> Vec<u8> {
        vec![0x0d, 0x0e, 0x0f, 0x1d, 0x1e, 0x1f, 0x24, 0x25, 0x26]
    }
}

impl<F> WitnessComponent<F> for BinaryExtensionSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }

    fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinaryExtensionSM as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
        }
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for BinaryExtensionSM {
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
        operation: ZiskRequiredOperation,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], drain, scope);
        result
    }
}
