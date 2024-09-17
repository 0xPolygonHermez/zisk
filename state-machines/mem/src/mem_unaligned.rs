use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::AbstractField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{MemUnalignedOp, OpResult, Provable};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct MemUnalignedSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<MemUnalignedOp>>,
}

#[allow(unused, unused_variables)]
impl MemUnalignedSM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let mem_aligned_sm =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let mem_aligned_sm = Arc::new(mem_aligned_sm);

        wcm.register_component(mem_aligned_sm.clone(), Some(airgroup_id), Some(air_ids));

        mem_aligned_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor<F: AbstractField>(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <MemUnalignedSM as Provable<MemUnalignedOp, OpResult, F>>::prove(
                self,
                &[],
                true,
                scope,
            );
        }
    }

    fn read(
        &self,
        _addr: u64,
        _width: usize, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((0, true))
    }

    fn write(
        &self,
        _addr: u64,
        _width: usize,
        _val: u64, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((0, true))
    }
}

impl<F> WitnessComponent<F> for MemUnalignedSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }
}

impl<F: AbstractField> Provable<MemUnalignedOp, OpResult, F> for MemUnalignedSM {
    fn calculate(&self, operation: MemUnalignedOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            MemUnalignedOp::Read(addr, width) => self.read(addr, width),
            MemUnalignedOp::Write(addr, width, val) => self.write(addr, width, val),
        }
    }

    fn prove(&self, operations: &[MemUnalignedOp], drain: bool, scope: &Scope) {
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
        operation: MemUnalignedOp,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = <MemUnalignedSM as Provable<MemUnalignedOp, (u64, bool), F>>::calculate(
            self,
            operation.clone(),
        );
        <MemUnalignedSM as Provable<MemUnalignedOp, (u64, bool), F>>::prove(
            self,
            &[operation],
            drain,
            scope,
        );
        result
    }
}
