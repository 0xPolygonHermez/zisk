use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::{ARITH64_AIR_IDS, ARITH_AIRGROUP_ID};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct Arith64SM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

impl Arith64SM {
    pub fn new<F>(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let arith64_sm =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let arith64_sm = Arc::new(arith64_sm);

        wcm.register_component(arith64_sm.clone(), Some(ARITH_AIRGROUP_ID), Some(ARITH64_AIR_IDS));

        arith64_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor<F: Field>(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <Arith64SM as Provable<ZiskRequiredOperation, OpResult>>::prove(self, &[], true, scope);
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb8, 0xb9, 0xba, 0xbb]
    }
}

impl<F> WitnessComponent<F> for Arith64SM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx<F>>,
        _sctx: Arc<SetupCtx<F>>,
    ) {
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for Arith64SM {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = ZiskOp::execute(operation.opcode, operation.a, operation.b);
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
