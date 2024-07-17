use log::debug;
use std::{cell::RefCell, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::WCComponent;

pub struct MemAlignedSM {}

#[allow(unused, unused_variables)]
impl MemAlignedSM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_ids: &[usize]) -> Rc<Self> {
        let mem_aligned_sm = Rc::new(MemAlignedSM {});
        wcm.register_component(Rc::clone(&mem_aligned_sm) as Rc<dyn WCComponent<F>>, Some(air_ids));

        mem_aligned_sm
    }

    pub fn read<F>(&self, _addr: u64, _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) -> u64 {
        0
    }

    pub fn write<F>(&self, _addr: u64, _val: u64, _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {}
}

impl<F> WCComponent<F> for MemAlignedSM {
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
