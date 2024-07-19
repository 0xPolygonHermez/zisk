use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx, WCPilout};
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use wchelpers::{WCLibrary, WCExecutor};
use proofman::WCManager;

use crate::{FibonacciSquare, FibonacciVadcopPilout, Module};

pub struct FibonacciVadcop<F> {
    pub wcm: WCManager<F>,
    pub fibonacci: Rc<FibonacciSquare>,
    pub module: Rc<Module>,
}

impl<F: AbstractField> FibonacciVadcop<F> {
    pub fn new() -> Self {
        let mut wcm = WCManager::new();

        let module = Module::new(&mut wcm);
        let fibonacci = FibonacciSquare::new(&mut wcm, &module);

        FibonacciVadcop { wcm, fibonacci, module }
    }
}

impl<F> WCLibrary<F> for FibonacciVadcop<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        self.wcm.start_proof(pctx, ectx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn start_execute(&mut self, _pctx: &mut ProofCtx<F>, _ectx: &mut ExecutionCtx) {}

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        self.fibonacci.execute(pctx, ectx);
    }

    fn end_execute(&mut self, _pctx: &mut ProofCtx<F>, _ectx: &mut ExecutionCtx) {}

    fn calculate_plan(&mut self, ectx: &mut ExecutionCtx) {
        self.wcm.calculate_plan(ectx);
    }

    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        self.wcm.calculate_witness(stage, pctx, ectx);
    }

    fn pilout(&self) -> WCPilout {
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
