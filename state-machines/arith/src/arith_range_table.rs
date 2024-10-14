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
const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct ArithRangeTableSM<F> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,

    _phantom: std::marker::PhantomData<F>,
}

impl<F: Field> ArithRangeTableSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let arith_range_table_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            inputs: Mutex::new(Vec::new()),
            _phantom: std::marker::PhantomData,
        };
        let arith_range_table_sm = Arc::new(arith_range_table_sm);

        wcm.register_component(arith_range_table_sm.clone(), Some(airgroup_id), Some(air_ids));

        arith_range_table_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <ArithRangeTableSM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
        }
    }

    pub fn operations() -> Vec<u8> {
        // TODO: use constants
        vec![0xb6, 0xb7, 0xbe, 0xbf]
    }
}

impl<F: Field> WitnessComponent<F> for ArithRangeTableSM<F> {
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

impl<F> Provable<ZiskRequiredOperation, OpResult> for ArithRangeTableSM<F> {
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
