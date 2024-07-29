use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx, WCPilout};
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::WCManager;
use sm_main::MainSM;
use sm_mem::MemSM;
use wchelpers::{WCExecutor, WCLibrary, WCOpCalculator};

use crate::ZiskPilout;

pub struct ZiskWC<F> {
    pub wcm: WCManager<F>,
    pub main_sm: Rc<MainSM>,
    pub mem_sm: Rc<MemSM>,
}

impl<F: AbstractField> Default for ZiskWC<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: AbstractField> ZiskWC<F> {
    pub fn new() -> Self {
        let mut wcm = WCManager::new();

        let mem_sm = MemSM::new(&mut wcm);

        let sm = vec![Rc::clone(&mem_sm) as Rc<dyn WCOpCalculator>];
        let main_sm = MainSM::new(&mut wcm, sm);

        ZiskWC { wcm, main_sm, mem_sm }
    }
}

impl<F> WCLibrary<F> for ZiskWC<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        self.wcm.start_proof(pctx, ectx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn calculate_plan(&mut self, ectx: &mut ExecutionCtx) {
        self.wcm.calculate_plan(ectx);
    }

    // fn start_execute(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
    //     self.wcm.start_execute(pctx, ectx);
    // }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        self.main_sm.execute(pctx, ectx);
    }

    // fn end_execute(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
    //     self.wcm.end_execute(pctx, ectx);
    // }

    // fn initialize_air_instances(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
    //     for id in ectx.owned_instances.iter() {
    //         pctx.air_instances.push((&ectx.instances[*id]).into());
    //     }
    // }

    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        self.wcm.calculate_witness(stage, pctx, ectx);
    }

    fn pilout(&self) -> WCPilout {
        ZiskPilout::get_pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library() -> Box<dyn WCLibrary<Goldilocks>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    Box::new(ZiskWC::new())
}
