use log::debug;
use sm_arith::ArithSM;
use std::{collections::HashMap, sync::Arc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use sm_mem::MemSM;
use wchelpers::{WCComponent, WCExecutor, WCOpCalculator};

pub struct MainSM {
    arith_sm: Arc<ArithSM>,
    mem_sm: Arc<MemSM>,
}

impl MainSM {
    pub fn new<F>(
        wcm: &mut WCManager<F>,
        mem_sm: Arc<MemSM>,
        arith_sm: Arc<ArithSM>,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let main = Arc::new(Self { mem_sm, arith_sm });

        wcm.register_component(main.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

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

    fn suggest_plan(&self, _ectx: &mut ExecutionCtx) {}
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
