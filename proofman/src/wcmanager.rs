use std::{collections::HashMap, error::Error, rc::Rc};

use log::info;

use common::{ExecutionCtx, ProofCtx};
use wchelpers::{WCComponent, WCExecutor};

use crate::{DefaultPlanner, Planner};

pub struct WCManager<F> {
    components: Vec<Rc<dyn WCComponent<F>>>,
    executors: Vec<Rc<dyn WCExecutor<F>>>,
    airs: HashMap<usize, Rc<dyn WCComponent<F>>>,
    planner: Box<dyn Planner<F>>,
    on_execute: Option<fn(&Self, &mut ProofCtx<F>, &mut ExecutionCtx)>,
}

impl<F> WCManager<F> {
    const MY_NAME: &'static str = "WCMnager";

    pub fn new() -> Self {
        WCManager {
            components: Vec::new(),
            executors: Vec::new(),
            airs: HashMap::new(),
            planner: Box::new(DefaultPlanner),
            on_execute: None,
        }
    }

    pub fn register_component(&mut self, component: Rc<dyn WCComponent<F>>) {
        self.components.push(component);
    }

    pub fn register_executor(&mut self, executor: Rc<dyn WCExecutor<F>>) {
        self.executors.push(executor);
    }

    pub fn register_airs(&mut self, air_ids: &[usize], air: Rc<dyn WCComponent<F>>) -> Result<(), Box<dyn Error>> {
        for air_id in air_ids.iter() {
            self.register_air(*air_id, air.clone())?;
        }

        Ok(())
    }

    pub fn register_air(&mut self, air_id: usize, air: Rc<dyn WCComponent<F>>) -> Result<(), Box<dyn Error>> {
        if self.airs.contains_key(&air_id) {
            return Err(format!("{}: AIR with ID {} is already registered", Self::MY_NAME, air_id).as_str().into());
        }

        self.airs.insert(air_id, air);
        Ok(())
    }

    pub fn set_planner(&mut self, planner: Box<dyn Planner<F>>) {
        self.planner = planner;
    }

    pub fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        info!("{}: Starting proof", Self::MY_NAME);
        for component in self.components.iter() {
            component.start_proof(pctx, ectx);
        }

        for component in self.components.iter() {
            component.start_execute(pctx, ectx);
        }

        Self::execute(self, pctx, ectx);
    }

    pub fn end_proof(&mut self) {
        info!("{}: Ending proof", Self::MY_NAME);
        for component in self.components.iter() {
            component.end_execute();
        }

        for component in self.components.iter() {
            component.end_proof();
        }
    }

    pub fn calculate_plan(&self, ectx: &mut ExecutionCtx) {
        self.planner.calculate_plan(&self.components, ectx);
    }

    pub fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        info!("{}: Calculating witness (stage {})", Self::MY_NAME, stage);

        for air_instance_ctx in ectx.instances.iter().rev() {
            let component = self.airs.get(&air_instance_ctx.air_id).unwrap();
            component.calculate_witness(stage, air_instance_ctx, pctx, ectx);
        }
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        info!("{}: Executing", Self::MY_NAME);
        if let Some(on_execute) = self.on_execute {
            on_execute(self, pctx, ectx);
        }
    }

    pub fn on_execute(&mut self, closure: fn(&Self, &mut ProofCtx<F>, &mut ExecutionCtx)) {
        self.on_execute = Some(closure);
    }
}
