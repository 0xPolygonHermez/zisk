use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};

use p3_field::Field;
use zisk_pil::{ARITH3264_AIR_IDS, ARITH_AIRGROUP_ID};
const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct Arith3264SM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

impl Arith3264SM {
    pub fn new<F>(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let arith3264_sm =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let arith3264_sm = Arc::new(arith3264_sm);

        wcm.register_component(
            arith3264_sm.clone(),
            Some(ARITH_AIRGROUP_ID),
            Some(ARITH3264_AIR_IDS),
        );

        arith3264_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor<F: Field>(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <Arith3264SM as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
        }
    }
}

impl<F> WitnessComponent<F> for Arith3264SM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for Arith3264SM {
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
                if drain && !inputs.is_empty() {
                    // println!("Arith3264SM: Draining inputs3264");
                }

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
