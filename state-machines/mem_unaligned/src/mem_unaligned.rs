use log::debug;
use std::{cell::RefCell, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::WCComponent;

pub struct MemUnalignedSM {}

#[allow(unused, unused_variables)]
impl MemUnalignedSM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Rc<Self> {
        let mem_unaligned_sm = Rc::new(MemUnalignedSM {});
        wcm.register_component(Rc::clone(&mem_unaligned_sm) as Rc<dyn WCComponent<F>>, Some(air_ids));

        mem_unaligned_sm
    }

    pub fn read<F>(&self, addr: u64, width: usize, ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) -> u64 {
        0
    }

    pub fn write<F>(&self, addr: u64, width: usize, val: u64, ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {}
}

impl<F> WCComponent<F> for MemUnalignedSM {
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
