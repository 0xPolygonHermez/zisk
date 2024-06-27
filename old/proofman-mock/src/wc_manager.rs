use std::collections::HashMap;

use crate::{AirPlanner, AirWitnessComputation, ProofContext, SubproofPlan, ProofWitnessComputation};

pub struct WCManager<T, I> {
    pub wc_modules: HashMap<String, Box<dyn AirWitnessComputation<T, I>>>,
    pub planner: Box<dyn AirPlanner>,
}

impl<T, I> WCManager<T, I> {
    pub fn new(planner: Box<dyn AirPlanner>) -> Self {
        let wc_modules: HashMap<String, Box<dyn AirWitnessComputation<T, I>>> = HashMap::new();

        Self { wc_modules, planner }
    }

    pub fn add_modules(&mut self, modules: HashMap<String, Box<dyn AirWitnessComputation<T, I>>>) {
        self.wc_modules.extend(modules);
    }

    pub fn add_module(&mut self, name: String, module: Box<dyn AirWitnessComputation<T, I>>) {
        self.wc_modules.insert(name, module);
    }

    pub fn get_module(&self, name: &str) -> Option<&Box<dyn AirWitnessComputation<T, I>>> {
        self.wc_modules.get(name)
    }
}

impl<T, I> ProofWitnessComputation<T, I> for WCManager<T, I> {
    fn start_proof(&self, _proof_id: &str, _instance: SubproofPlan) {
        unimplemented!();
    }

    fn witness_calculate(&self, _stage_id: u32, _proof_ctx: ProofContext<T>, _inputs: Option<I>) {
        unimplemented!();
    }

    fn end_proof(&self, _proof_id: &str) {
        unimplemented!();
    }
}
