use std::rc::Rc;

use log::info;

use common::{ExecutionCtx, ProofCtx};
use wchelpers::{WCComponent, WCExecutor};

use crate::{DefaultPlanner, Planner};
use common::Prover;

pub struct WCManager<F> {
    components: Vec<Rc<dyn WCComponent<F>>>,
    executors: Vec<Rc<dyn WCExecutor<F>>>,
    planner: Box<dyn Planner<F>>,
}

impl<F> WCManager<F> {
    const MY_NAME: &'static str = "WCMnager";

    pub fn new() -> Self {
        WCManager { components: Vec::new(), executors: Vec::new(), planner: Box::new(DefaultPlanner) }
    }

    pub fn register_component(&mut self, component: Rc<dyn WCComponent<F>>) {
        self.components.push(component);
    }

    pub fn register_executor(&mut self, executor: Rc<dyn WCExecutor<F>>) {
        self.executors.push(executor);
    }

    pub fn set_planner(&mut self, planner: Box<dyn Planner<F>>) {
        self.planner = planner;
    }

    pub fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        info!("{}: Starting proof", Self::MY_NAME);
        for component in self.components.iter() {
            component.start_proof(pctx, ectx);
        }

        Self::execute(self, pctx, ectx);
    }

    pub fn end_proof(&mut self) {
        info!("{}: Ending proof", Self::MY_NAME);
        for component in self.components.iter() {
            component.end_proof();
        }
    }

    pub fn calculate_plan(&self, ectx: &mut ExecutionCtx) {
        self.planner.calculate_plan(&self.components, ectx);
    }

    pub fn calculate_witness(
        &self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        provers: &Vec<Box<dyn Prover<F>>>,
    ) {
        info!("{}: Calculating witness (stage {})", Self::MY_NAME, stage);

        for air_instance_ctx in ectx.instances.iter().rev() {
            let component = &self.components[air_instance_ctx.wc_component_idx.unwrap()];
            component.calculate_witness(stage, air_instance_ctx, pctx, ectx, provers);
        }
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        info!("{}: Executing", Self::MY_NAME);
        for executor in self.executors.iter() {
            executor.execute(pctx, ectx);
        }
    }
}
