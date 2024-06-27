use std::{collections::HashMap, error::Error};

use pilout::pilout_proxy::PilOutProxy;
use proofman_common::{AirInstanceWitnessComputation, ExecutionCtx, WitnessExecutor, ProofCtx};

use super::{arith::ArithSM, binary::BinarySM, main::MainSM, mem::MemSM};

#[allow(dead_code)]
pub struct ZiskProcessor<F> {
    modules: HashMap<String, Box<dyn AirInstanceWitnessComputation<F>>>,
    executors: Vec<String>,
}

#[allow(dead_code, unused_variables)]
impl<F: 'static> ZiskProcessor<F> {
    pub fn new(pilout: &PilOutProxy) -> Self {
        let mut modules: HashMap<String, Box<dyn AirInstanceWitnessComputation<F>>> = HashMap::new();

        modules.insert("Main".into(), Box::new(MainSM::<F>::new()));
        modules.insert("Arith".into(), Box::new(ArithSM::<F>::new()));
        modules.insert("Binary".into(), Box::new(BinarySM::<F>::new()));
        modules.insert("Memory".into(), Box::new(MemSM::<F>::new()));

        let executors = vec!["Main".into()];

        Self { modules, executors }
    }

    pub fn add_module(
        &mut self,
        name: String,
        module: Box<dyn AirInstanceWitnessComputation<F>>,
        is_executor: bool,
    ) -> Result<(), Box<dyn Error>> {
        if self.modules.contains_key(&name) {
            return Err("Module already exists".into());
        }
        self.modules.insert(name.clone(), module);

        if is_executor {
            self.executors.push(name.clone());
        }

        Ok(())
    }

    pub fn get_modules(&self) -> &HashMap<String, Box<dyn AirInstanceWitnessComputation<F>>> {
        &self.modules
    }

    pub fn execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        for executor_name in self.executors.iter() {
            let module = self.modules.get_mut(executor_name).unwrap_or_else(|| {
                panic!("Failed to get module");
            });
            let executor =
                (&mut *module as &mut dyn std::any::Any).downcast_mut::<Box<dyn WitnessExecutor<F>>>().unwrap();

            executor.start_execute(proof_ctx, execution_ctx);

            executor.execute(proof_ctx, execution_ctx);

            executor.end_execute(proof_ctx, execution_ctx);
        }
    }
}
