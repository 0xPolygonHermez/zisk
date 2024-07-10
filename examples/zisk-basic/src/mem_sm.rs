use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx};
use wchelpers::WCComponent;
use proofman::WCManager;

pub struct MemSM;

impl MemSM {
    const MY_NAME: &'static str = "MemSM    ";

    pub fn new<F>(wcm: &mut WCManager<F>) -> Rc<Self> {
        let mem_sm = Rc::new(MemSM);
        wcm.register_component(Rc::clone(&mem_sm) as Rc<dyn WCComponent<F>>);

        mem_sm
    }
}

impl<F> WCComponent<F> for MemSM {
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
