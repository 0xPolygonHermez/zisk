use std::sync::Arc;

use common::{ExecutionCtx, ProofCtx, WCPilout};
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::WCManager;
use sm_arith::ArithSM;
use sm_arith_32::Arith32SM;
use sm_arith_3264::Arith3264SM;
use sm_arith_64::Arith64SM;
use sm_main::MainSM;
use sm_mem::MemSM;
use sm_mem_aligned::MemAlignedSM;
use sm_mem_unaligned::MemUnalignedSM;
use wchelpers::{WCExecutor, WCLibrary};

use crate::{
    Pilout, ARITH3264_AIR_IDS, ARITH32_AIR_IDS, ARITH64_AIR_IDS, MAIN_AIR_IDS, MEM_ALIGN_AIR_IDS,
    MEM_UNALIGNED_AIR_IDS,
};

pub struct ZiskWC<F> {
    pub wcm: WCManager<F>,
    // pub buffer_allocator: Box<BufferAllocator>,
    pub main_sm: Arc<MainSM>,
    pub mem_sm: Arc<MemSM>,
    pub mem_aligned_sm: Arc<MemAlignedSM>,
    pub mem_unaligned_sm: Arc<MemUnalignedSM>,
    pub arith_sm: Arc<ArithSM>,
    pub arith_32_sm: Arc<Arith32SM>,
}

impl<F: AbstractField> Default for ZiskWC<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: AbstractField> ZiskWC<F> {
    pub fn new(/*buffer_allocator: Box<BufferAllocator>*/) -> Self {
        let mut wcm = WCManager::new();

        let mem_aligned_sm = MemAlignedSM::new(&mut wcm, MEM_ALIGN_AIR_IDS);
        let mem_unaligned_sm = MemUnalignedSM::new(&mut wcm, MEM_UNALIGNED_AIR_IDS);
        let mem_sm = MemSM::new(&mut wcm, mem_aligned_sm.clone(), mem_unaligned_sm.clone());

        let arith_32_sm = Arith32SM::new(&mut wcm, ARITH32_AIR_IDS);
        let arith_64_sm = Arith64SM::new(&mut wcm, ARITH64_AIR_IDS);
        let arith_3264_sm = Arith3264SM::new(&mut wcm, ARITH3264_AIR_IDS);
        let arith_sm =
            ArithSM::new(&mut wcm, arith_32_sm.clone(), arith_64_sm.clone(), arith_3264_sm.clone());

        let main_sm = MainSM::new(&mut wcm, mem_sm.clone(), arith_sm.clone(), MAIN_AIR_IDS);

        ZiskWC {
            wcm,
            // buffer_allocator,
            main_sm,
            mem_sm,
            mem_aligned_sm,
            mem_unaligned_sm,
            arith_sm,
            arith_32_sm,
        }
    }
}

impl<F> WCLibrary<F> for ZiskWC<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        self.wcm.start_proof(pctx, ectx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        self.main_sm.execute(pctx, ectx);
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
pub extern "Rust" fn init_library(/*buffer_allocator: Box<BufferAllocator>,*/
) -> Box<dyn WCLibrary<Goldilocks>> {
    Box::new(ZiskWC::new(/*buffer_allocator*/))
}