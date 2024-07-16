use log::debug;
use std::{cell::RefCell, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCOpCalculator};

pub struct MemSM {
    inputs: RefCell<Vec<(u64, u64)>>,
}

impl MemSM {
    pub fn new<F>(wcm: &mut WCManager<F>, air_id: usize) -> Rc<Self> {
        let mem_sm = Rc::new(MemSM { inputs: RefCell::new(Vec::new()) });
        wcm.register_component(Rc::clone(&mem_sm) as Rc<dyn WCComponent<F>>);
        wcm.register_air(air_id, Rc::clone(&mem_sm) as Rc<dyn WCComponent<F>>);

        mem_sm
    }

    pub fn read(&self, ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx, addr: u64) -> u64 {

    }

    pub fn write(&self, ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx, addr: u64, val: u64) {

    }

}

impl<F> WCComponent<F> for MemSM {
    fn calculate_witness(&self, stage: u32, air_instance: &AirInstance, pctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {}

    fn calculate_plan(&self, ectx: &mut ExecutionCtx) {}
}
