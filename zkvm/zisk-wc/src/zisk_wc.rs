use std::{borrow::Borrow, rc::Rc};

use common::{ExecutionCtx, ProofCtx, WCPilOut};
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::WCManager;
use wchelpers::{WCComponent, WCLibrary, WCOpCalculator};
use zisk_sm::{MainSM, MemSM};

use crate::FibonacciVadcopPilout;

pub struct FibonacciVadcop<F> {
    pub wcm: WCManager<F>,
    pub main_sm: Rc<MainSM>,
    pub mem_sm: Rc<MemSM>,
}

impl<F: AbstractField> FibonacciVadcop<F> {
    pub fn new() -> Self {
        let mut wcm = WCManager::new();

        let mem_sm = MemSM::new(&mut wcm);

        let sm = vec![Rc::clone(&mem_sm) as Rc<dyn WCOpCalculator>];
        let main_sm = MainSM::new(&mut wcm, sm);

        FibonacciVadcop {
            wcm,
            main_sm,
            mem_sm,
        }
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
