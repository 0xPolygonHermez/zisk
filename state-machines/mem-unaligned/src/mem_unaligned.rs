use std::{
    mem,
    sync::{Arc, Mutex},
};

use proofman::WitnessManager;
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_common::{MemUnalignedOp, OpResult, Provable};
use witness_helpers::WitnessComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 3;

pub struct MemUnalignedSM {
    inputs: Mutex<Vec<MemUnalignedOp>>,
}

#[allow(unused, unused_variables)]
impl MemUnalignedSM {
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
        _air_instance: &AirInstance,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
    ) {
    }

    fn suggest_plan(&self, _ectx: &mut ExecutionCtx) {}
}

impl Provable<MemUnalignedOp, OpResult> for MemUnalignedSM {
    fn calculate(&self, operation: MemUnalignedOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        match operation {
            MemUnalignedOp::Read(addr, width) => self.read(addr, width),
            MemUnalignedOp::Write(addr, width, val) => self.write(addr, width, val),
        }
    }

    fn prove(&self, operations: &[MemUnalignedOp], is_last: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);
            if is_last || inputs.len() >= PROVE_CHUNK_SIZE {
                let _inputs = mem::take(&mut *inputs);

                scope.spawn(move |_scope| {
                    println!(
                        "Arith32: Proving [{:?}..{:?}]",
                        _inputs[0],
                        _inputs[_inputs.len() - 1]
                    );
                    println!("Arith32: Finishing the worker thread");
                });
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: MemUnalignedOp,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
