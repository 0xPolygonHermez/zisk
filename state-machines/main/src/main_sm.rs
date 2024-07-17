use log::debug;
use std::{collections::HashMap, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCExecutor, WCOpCalculator};
use sm_mem::MemSM;

pub struct MainSM {
    mem: Rc<MemSM>,
}

impl MainSM {
    pub fn new<F>(wcm: &mut WCManager<F>, mem: Rc<MemSM>, air_ids: &[usize]) -> Rc<Self> {
        let main = Rc::new(Self { mem });

        wcm.register_component(Rc::clone(&main) as Rc<dyn WCComponent<F>>);
        wcm.register_executor(Rc::clone(&main) as Rc<dyn WCExecutor<F>>);
        wcm.register_airs(air_ids, Rc::clone(&main) as Rc<dyn WCComponent<F>>)
            .expect(format!("Failed to register AIRs {:?}", air_ids).as_str());

        main
    }

    pub fn execute<F>(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {}
}

impl<F> WCComponent<F> for MainSM {
    fn calculate_witness(&self, stage: u32, air_instance: &AirInstance, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {}

    fn suggest_plan(&self, ectx: &mut ExecutionCtx) {}
}

impl<F> WCExecutor<F> for MainSM {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        let mut end = false;

        let mem = self.mem.as_ref();
        while (!end) {
            let addr = 3;
            // let val = mem.read(addr, pctx, ectx);
        }
    }
}
