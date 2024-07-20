use log::debug;
use std::{collections::HashMap, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use sm_mem::MemSM;
use wchelpers::{WCComponent, WCExecutor, WCOpCalculator};

pub struct MainSM {
    mem: Rc<MemSM>,
}

impl MainSM {
    pub fn new<F>(wcm: &mut WCManager<F>, mem: Rc<MemSM>, air_ids: &[usize]) -> Rc<Self> {
        let main = Rc::new(Self { mem });

        wcm.register_component(Rc::clone(&main) as Rc<dyn WCComponent<F>>, Some(air_ids));

        main
    }

    pub fn execute<F>(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {}
}

impl<F> WCComponent<F> for MainSM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {
    }

    fn suggest_plan(&self, ectx: &mut ExecutionCtx) {}
}

impl<F> WCExecutor<F> for MainSM {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        // let mut end = false;

        // let mem = self.mem.as_ref();
        // while !end {
        //     let addr = 3;
        //     // let val = mem.read(addr, pctx, ectx);
        // }
    }
}
