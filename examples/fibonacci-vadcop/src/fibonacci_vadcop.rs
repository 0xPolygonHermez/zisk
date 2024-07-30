use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx, WCPilOut};
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use wchelpers::{WCComponent, WCLibrary};
use proofman::WCManager;
use common::Prover;

use crate::{FibonacciSquare, FibonacciVadcopPilout, Module};

pub struct FibonacciVadcop<F> {
    pub wcm: WCManager<F>,
    pub fibonacci: Rc<FibonacciSquare>,
    pub module: Rc<Module>,
}

impl<F: AbstractField> FibonacciVadcop<F> {
    pub fn new() -> Self {
        let mut wcm = WCManager::new();

        let module = Module::new_no_register(&mut wcm);
        let fibonacci = FibonacciSquare::new(&mut wcm, &module);
        // Register the module component after the fibonacci component
        wcm.register_component(Rc::clone(&module) as Rc<dyn WCComponent<F>>);
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

    fn calculate_plan(&mut self, ectx: &mut ExecutionCtx) {
        self.wcm.calculate_plan(ectx);
    }

    fn initialize_air_instances(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        for id in ectx.owned_instances.iter() {
            pctx.air_instances.push((&ectx.instances[*id]).into());
        }
    }
    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        provers: &Vec<Box<dyn Prover<F>>>,
    ) {
        self.wcm.calculate_witness(stage, pctx, ectx, provers);
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
