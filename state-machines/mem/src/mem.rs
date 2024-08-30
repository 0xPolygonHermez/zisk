use crate::{MemAlignedSM, MemUnalignedSM};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{MemOp, MemUnalignedOp, OpResult, Provable};
use std::sync::{Arc, Mutex};
use zisk_core::ZiskRequiredMemory;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};

#[allow(dead_code)]
const PROVE_CHUNK_SIZE: usize = 1 << 3;

#[allow(dead_code)]
pub struct MemSM {
    inputs_aligned: Mutex<Vec<MemOp>>,
    inputs_unaligned: Mutex<Vec<MemUnalignedOp>>,
    mem_aligned_sm: Arc<MemAlignedSM>,
    mem_unaligned_sm: Arc<MemUnalignedSM>,
}

impl MemSM {
    pub fn new<F>(
        wcm: &mut WitnessManager<F>,
        mem_aligned_sm: Arc<MemAlignedSM>,
        mem_unaligned_sm: Arc<MemUnalignedSM>,
    ) -> Arc<Self> {
        let mem_sm = Self {
            inputs_aligned: Mutex::new(Vec::new()),
            inputs_unaligned: Mutex::new(Vec::new()),
            mem_aligned_sm,
            mem_unaligned_sm,
        };
        let mem_sm = Arc::new(mem_sm);

        wcm.register_component(mem_sm.clone() as Arc<dyn WitnessComponent<F>>, None);

        mem_sm
    }
}

impl<F> WitnessComponent<F> for MemSM {
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

impl Provable<ZiskRequiredMemory, OpResult> for MemSM {
    fn calculate(
        &self,
        _operation: ZiskRequiredMemory,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        unimplemented!()
    }

    fn prove(&self, _operations: &[ZiskRequiredMemory], _drain: bool, _scope: &Scope) {}

    fn calculate_prove(
        &self,
        _operation: ZiskRequiredMemory,
        _drain: bool,
        _scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        unimplemented!()
    }
}
