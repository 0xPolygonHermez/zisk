use log::debug;
use std::{cell::RefCell, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCOpCalculator};

pub struct MemSM {
    inputs: RefCell<Vec<(u64, u64)>>,
}

impl MemSM {
    pub fn new<F>(wcm: &mut WCManager<F>) -> Rc<Self> {
        let mem_sm = Rc::new(MemSM { inputs: RefCell::new(Vec::new()) });
        wcm.register_component(Rc::clone(&mem_sm) as Rc<dyn WCComponent<F>>);

        mem_sm
    }
}

impl WCOpCalculator for MemSM {
    // 0:x, 1:module
    fn calculate_verify(&self, verify: bool, values: Vec<u64>) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
        let (x, module) = (values[0], values[1]);

        let x_mod = x % module;

        if verify {
            self.inputs.borrow_mut().push((x.into(), x_mod.into()));
        }

        Ok(vec![x_mod])
    }

    fn codes(&self) -> Vec<&str> {
        vec!["mOp"]
    }
}

impl<F> WCComponent<F> for MemSM {
    fn calculate_witness(&self, stage: u32, air_instance: &AirInstance, pctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {}

    fn calculate_plan(&self, ectx: &mut ExecutionCtx) {}
}
