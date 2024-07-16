use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx, WCPilOut};
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::WCManager;
use sm_main::MainSM;
use sm_mem::MemSM;
use wchelpers::{WCLibrary, WCOpCalculator};

use crate::FibonacciVadcopPilout;

pub struct ZiskWC<F> {
    pub wcm: WCManager<F>,
    pub main_sm: Rc<MainSM>,
    pub mem_sm: Rc<MemSM>,
}

impl<F: AbstractField> ZiskWC<F> {
    pub fn new() -> Self {
        let mut wcm = WCManager::new();

        let mem_aligned_sm = MemSM::new(&mut wcm, pil_helpers::mem_aligned::air_id);
        let mem_unaligned_sm = MemSM::new(&mut wcm, pil_helpers::mem_unaligned::air_id);
        let mem_sm = MemSM::new(&mut wcm, mem_aligned_sm, mem_unaligned_sm);

        let main_sm = MainSM::new(&mut wcm, mem_sm);

        ZiskWC { wcm, main_sm, mem_sm }
    }
}

impl<F> WCLibrary<F> for FibonacciVadcop<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        self.wcm.start_proof(pctx, ectx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn calculate_plan(&mut self, ectx: &mut ExecutionCtx) {
        self.wcm.calculate_plan(ectx);
    }

    fn initialize_air_instances(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        for id in ectx.owned_instances.iter() {
            pctx.air_instances.push((&ectx.instances[*id]).into());
        }
    }
    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        self.wcm.calculate_witness(stage, pctx, ectx);
    }

    fn get_pilout(&self) -> WCPilOut {
        FibonacciVadcopPilout::get_fibonacci_vadcop_pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library<'a>() -> Box<dyn WCLibrary<Goldilocks>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    Box::new(FibonacciVadcop::new())
}
