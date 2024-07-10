use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx};
use wchelpers::WCLibrary;
use proofman::WCManager;

use crate::{MainSM, MemSM};

pub struct Zisk<F> {
    pub wcm: WCManager<F>,
    pub main_sm: Rc<MainSM>,
    pub mem_sm: Rc<MemSM>,
}

impl<F> Zisk<F> {
    pub fn new() -> Self {
        let mut wcm = WCManager::new();

        let mem_sm = MemSM::new(&mut wcm);
        let main_sm = MainSM::new(&mut wcm, &mem_sm);

        Zisk {
            wcm,
            main_sm,
            mem_sm,
        }
    }
}

impl<F> WCLibrary<F> for Zisk<F> {
    fn start_proof(&mut self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx) {
        self.wcm.start_proof(pctx, ectx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn calculate_plan(&mut self, _pctx: &ProofCtx<F>) {
        self.wcm.calculate_plan();
    }

    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        self.wcm.calculate_witness(stage, pctx, ectx);
    }
}

// This is a mock for Goldilocks type
type GL = usize;

#[no_mangle]
pub extern "Rust" fn init_library<'a>() -> Box<dyn WCLibrary<GL>> {
    Box::new(Zisk::new())
}
