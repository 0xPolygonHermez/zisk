use log::debug;
use std::{collections::HashMap, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCExecutor, WCOpCalculator};

pub struct MainSM {
    sm: HashMap<String, Rc<dyn WCOpCalculator>>,
}

impl MainSM {
    pub fn new<F>(wcm: &mut WCManager<F>, sm: Vec<Rc<dyn WCOpCalculator>>) -> Rc<Self> {
        let mut _sm = HashMap::new();

        for s in sm {
            for code in s.codes() {
                _sm.insert(code.to_string(), Rc::clone(&s));
            }
        }
        Rc::new(Self { sm: _sm })

        // wcm.register_component(Rc::clone(&main) as Rc<dyn WCComponent<F>>);
        // wcm.register_executor(Rc::clone(&main) as Rc<dyn WCExecutor<F>>);
    }
}

impl<F> WCComponent<F> for MainSM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {
    }

    fn suggest_plan(&self, ectx: &mut ExecutionCtx) {}
}

impl<F> WCExecutor<F> for MainSM {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {}
}
