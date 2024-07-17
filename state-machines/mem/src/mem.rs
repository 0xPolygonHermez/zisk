use log::debug;
use sm_mem_aligned::MemAlignedSM;
use sm_mem_unaligned::MemUnalignedSM;
use std::{cell::RefCell, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCOpCalculator};

pub struct MemSM {
    mem_aligned: Rc<MemAlignedSM>,
    mem_unaligned: Rc<MemUnalignedSM>,
}

impl MemSM {
    pub fn new<F>(
        wcm: &mut WCManager<F>,
        mem_aligned: Rc<MemAlignedSM>,
        mem_unaligned: Rc<MemUnalignedSM>,
    ) -> Rc<Self> {
        let mem_sm = Rc::new(Self { mem_aligned, mem_unaligned });
        wcm.register_component(Rc::clone(&mem_sm) as Rc<dyn WCComponent<F>>);

        mem_sm
    }
}

impl<F> WCComponent<F> for MemSM {
    fn calculate_witness(&self, stage: u32, air_instance: &AirInstance, pctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {}

    fn suggest_plan(&self, ectx: &mut ExecutionCtx) {}
}
