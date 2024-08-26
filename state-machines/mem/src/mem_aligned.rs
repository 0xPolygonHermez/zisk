use std::{
    mem,
    sync::{Arc, Mutex},
};

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{MemOp, OpResult, Provable};

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct MemAlignedSM {
    inputs: Mutex<Vec<MemOp>>,
}

#[allow(unused, unused_variables)]
impl MemAlignedSM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let mem_aligned_sm = Self { inputs: Mutex::new(Vec::new()) };
        let mem_aligned_sm = Arc::new(mem_aligned_sm);

        wcm.register_component(
            mem_aligned_sm.clone() as Arc<dyn WitnessComponent<F>>,
            Some(air_ids),
        );

        mem_aligned_sm
    }

    fn read(
        &self,
        _addr: u64, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((0, true))
    }

    fn write(
        &self,
        _addr: u64,
        _val: u64, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((0, true))
    }
}

impl<F> WitnessComponent<F> for MemAlignedSM {
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

impl Provable<MemOp, OpResult> for MemAlignedSM {
    fn calculate(&self, operation: MemOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            MemOp::Read(addr) => self.read(addr),
            MemOp::Write(addr, val) => self.write(addr, val),
        }
    }

    fn prove(&self, operations: &[MemOp], is_last: bool, scope: &Scope) {
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
        operation: MemOp,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
