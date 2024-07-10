use std::rc::Rc;

use p3_field::PrimeField64;
use p3_goldilocks::Goldilocks;

use common::{ExecutionCtx, ProofCtx};
use wchelpers::WCLibrary;
use proofman::WCManager;

use crate::{Fibonacci, Module};

pub struct FibonacciVadcop<F> {
    pub wcm: WCManager<F>,
    pub fibonacci: Rc<Fibonacci>,
    pub module: Rc<Module>,
}

impl<F: PrimeField64> FibonacciVadcop<F> {
    pub fn new() -> Self {
        let mut wcm = WCManager::new();

        let fibonacci = Fibonacci::new(&mut wcm);
        let module = Module::new(&mut wcm);

        FibonacciVadcop { wcm, fibonacci, module }
    }
}

impl<F> WCLibrary<F> for FibonacciVadcop<F> {
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
type GL = Goldilocks;

#[no_mangle]
pub extern "Rust" fn init_library<'a>() -> Box<dyn WCLibrary<GL>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    Box::new(FibonacciVadcop::new())
}
