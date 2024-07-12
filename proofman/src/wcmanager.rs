use std::rc::Rc;

use log::info;

use common::{ExecutionCtx, ProofCtx};
use wchelpers::{WCComponent, WCExecutor};

pub struct WCManager<F> {
    components: Vec<Rc<dyn WCComponent<F>>>,
    executors: Vec<Rc<dyn WCExecutor<F>>>,
}

impl<F> WCManager<F> {
    const MY_NAME: &'static str = "WCManager";

    pub fn new() -> Self {
        WCManager { components: Vec::new(), executors: Vec::new() }
    }

    pub fn register_component(&mut self, component: Rc<dyn WCComponent<F>>) {
        self.components.push(component);
    }

    pub fn register_executor(&mut self, executor: Rc<dyn WCExecutor<F>>) {
        self.executors.push(executor);
    }

    pub fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        println!("{}: Starting proof", Self::MY_NAME);
        for component in self.components.iter() {
            component.start_proof(pctx, ectx);
        }

        Self::execute(self, pctx, ectx);
    }

    pub fn end_proof(&mut self) {
        println!("{}: Ending proof", Self::MY_NAME);
        for component in self.components.iter() {
            component.end_proof();
        }
    }

    pub fn calculate_plan(&self, ectx: &mut ExecutionCtx) {
        println!("{}: Calculating plan", Self::MY_NAME);
        let mut last_idx;
        for (component_idx, component) in self.components.iter().enumerate() {
            last_idx = ectx.instances.len();
            component.calculate_plan(ectx);
            for i in last_idx..ectx.instances.len() {
                ectx.instances[i].set_wc_component_idx(component_idx);
            }
        }

        ectx.owned_instances = (0..ectx.instances.len()).collect();
    }

    pub fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        info!("{}: Calculating witness (stage {})", Self::MY_NAME, stage);
        // for component in self.components.iter() {
        //     component.calculate_witness(stage, pctx, ectx);
        // }
        for air_instance_ctx in ectx.instances.iter().rev() {
            let component = &self.components[air_instance_ctx.get_wc_component_idx().unwrap()];
            component.calculate_witness(stage, air_instance_ctx, pctx, ectx);
        }
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        println!("{}: Executing", Self::MY_NAME);
        for executor in self.executors.iter() {
            executor.execute(pctx, ectx);
        }
    }
}
