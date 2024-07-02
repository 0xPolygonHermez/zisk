use std::{cell::RefCell, collections::HashMap, error::Error, rc::Rc};

use pilout::pilout_proxy::PilOutProxy;

extern crate common;
use common::{AirInstanceWitnessComputation, ExecutionCtx, ProofCtx, WitnessPilOut};

use crate::WitnessExecutor;

pub trait WitnessManagerT<F> {
    fn initialize(&self);

    fn get_pilout(&self) -> &PilOutProxy;

    fn start_proof(&mut self, proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx);

    fn end_proof(&mut self, proof_ctx: &ProofCtx<F>);

    fn calculate_air_instances_map(&self, proof_ctx: &ProofCtx<F>);

    fn calculate_witness(&self, stage: u32, pilout: &PilOutProxy, proof_ctx: &ProofCtx<F>);
}

pub struct WitnessManager<'a, F> {
    pilout: WitnessPilOut,
    modules: HashMap<String, Rc<RefCell<dyn AirInstanceWitnessComputation<'a, F> + 'a>>>,
    executors: HashMap<String, Rc<RefCell<dyn WitnessExecutor<'a, F> + 'a>>>,

    _phantom: std::marker::PhantomData<&'a F>,
}

impl<'a, F> WitnessManager<'a, F> {
    pub fn new(pilout: WitnessPilOut) -> Self {
        Self { pilout, modules: HashMap::new(), executors: HashMap::new(), _phantom: std::marker::PhantomData }
    }

    pub fn add_module(
        &mut self,
        name: String,
        module: Rc<RefCell<dyn AirInstanceWitnessComputation<'a, F> + 'a>>,
    ) -> Result<(), Box<dyn Error>> {
        if self.modules.contains_key(&name) {
            return Err("Module already exists".into());
        }
        self.modules.insert(name.clone(), module.clone());

        Ok(())
    }

    pub fn add_executor(
        &mut self,
        name: String,
        executor: Rc<RefCell<dyn WitnessExecutor<'a, F> + 'a>>,
    ) -> Result<(), Box<dyn Error>> {
        if self.executors.contains_key(&name) {
            return Err("Executor already exists".into());
        }
        self.executors.insert(name.clone(), executor.clone());
        Ok(())
    }
}

////////

pub trait WitnessModule<'a, F>: HasSubcomponents<'a, F> {
    fn start_proof(&self, proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        log::trace!("{}: ··· Starting proof", self.name());
        for subcomponent in self.get_subcomponents() {
            subcomponent.start_proof(proof_ctx, execution_ctx);
        }
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        log::trace!("{}: ··· Ending proof", self.name());
        for subcomponent in self.get_subcomponents() {
            subcomponent.end_proof(proof_ctx);
        }
    }

    fn _calculate_air_instances_map(&self, proof_ctx: &ProofCtx<F>) {
        log::trace!("{}: ··· Calculating Air instances map", self.name());
        self.calculate_air_instances_map(proof_ctx);

        for subcomponent in self.get_subcomponents() {
            subcomponent._calculate_air_instances_map(proof_ctx);
        }
    }

    fn _calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        log::trace!("{}: ··· Calculating Witness for stage {}", self.name(), stage);
        self.calculate_witness(stage, proof_ctx, execution_ctx);

        for subcomponent in self.get_subcomponents() {
            subcomponent._calculate_witness(stage, proof_ctx, execution_ctx);
        }
    }

    fn calculate_air_instances_map(&self, proof_ctx: &ProofCtx<F>);

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx);

    fn name(&self) -> String;
}

// Trait for components with subcomponents management
pub trait HasSubcomponents<'a, F> {
    fn add_subcomponent(&mut self, subcomponent: Box<dyn WitnessModule<'a, F> + 'a>);
    fn get_subcomponents(&self) -> &[Box<dyn WitnessModule<'a, F> + 'a>];
    fn get_subcomponents_mut(&mut self) -> &mut Vec<Box<dyn WitnessModule<'a, F> + 'a>>;
}
