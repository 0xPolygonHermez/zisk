use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx};
use wchelpers::{WCComponent, WCExecutor};
use proofman::WCManager;

use crate::MemSM;

#[allow(dead_code)]
pub struct MainSM {
    mem_sm: Rc<MemSM>,
}

impl MainSM {
    const MY_NAME: &'static str = "MainSM   ";

    pub fn new<F>(wcm: &mut WCManager<F>, mem_sm: &Rc<MemSM>) -> Rc<Self> {
        let main_sm = Rc::new(Self {
            mem_sm: Rc::clone(&mem_sm),
        });
        wcm.register_component(Rc::clone(&main_sm) as Rc<dyn WCComponent<F>>);
        wcm.register_executor(Rc::clone(&main_sm) as Rc<dyn WCExecutor<F>>);
        main_sm
    }

    fn start_execute<F>(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx) {
        println!("{}: Starting execute", Self::MY_NAME);
    }

    fn execute<F>(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx) {
        println!("{}: Executing", Self::MY_NAME);
    }

    fn end_execute<F>(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx) {
        println!("{}: Ending execute", Self::MY_NAME);
    }
}

impl<F> WCComponent<F> for MainSM {
    fn start_proof(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx) {
        println!("{}: Starting proof", Self::MY_NAME);
    }

    fn end_proof(&self) {
        println!("{}: Ending proof", Self::MY_NAME);
    }

    fn calculate_witness(&self, _stage: u32, _pctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {
        println!("{}: Calculating witness", Self::MY_NAME);
    }
}

impl<F> WCExecutor<F> for MainSM {
    fn execute(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx) {
        self.start_execute(pctx, ectx);
        self.execute(pctx, ectx);
        self.end_execute(pctx, ectx);
    }
}
