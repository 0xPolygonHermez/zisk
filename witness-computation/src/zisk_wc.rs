use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx, WCPilout};
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::WCManager;
use sm_main::MainSM;
use sm_mem::MemSM;
use sm_mem_aligned::MemAlignedSM;
use sm_mem_unaligned::MemUnalignedSM;
use wchelpers::WCLibrary;

use crate::{Pilout, MAIN_AIR_IDS, MEM_ALIGN_AIR_IDS, MEM_UNALIGNED_AIR_IDS};

pub struct ZiskWC<F> {
    pub wcm: WCManager<F>,
    pub main_sm: Rc<MainSM>,
    pub mem_sm: Rc<MemSM>,
    pub mem_aligned_sm: Rc<MemAlignedSM>,
    pub mem_unaligned_sm: Rc<MemUnalignedSM>,
}

impl<F: AbstractField> ZiskWC<F> {
    pub fn new(pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) -> Self {
        let mut wcm = WCManager::new();

        let mem_aligned_sm = MemAlignedSM::new(&mut wcm, MEM_ALIGN_AIR_IDS);
        let mem_unaligned_sm = MemUnalignedSM::new(&mut wcm, MEM_UNALIGNED_AIR_IDS);
        let mem_sm = MemSM::new(&mut wcm, mem_aligned_sm.clone(), mem_unaligned_sm.clone());

        let main_sm = MainSM::new(&mut wcm, mem_sm.clone(), MAIN_AIR_IDS);

        // wcm.on_execute(|main_sm: Rc<MainSM>, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx| {
        //     main_sm.execute(pctx, ectx);
        // });

        ZiskWC { wcm, main_sm, mem_sm, mem_aligned_sm, mem_unaligned_sm }
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

    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        self.wcm.calculate_witness(stage, pctx, ectx);
    }

    fn pilout(&self) -> WCPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    pctx: &mut ProofCtx<Goldilocks>,
    ectx: &ExecutionCtx,
) -> Box<dyn WCLibrary<Goldilocks>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    Box::new(ZiskWC::new(pctx, ectx))
}
