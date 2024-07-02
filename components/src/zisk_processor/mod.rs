use std::{cell::RefCell, collections::HashMap, error::Error, rc::Rc};

use log::trace;
use pilout::pilout_proxy::PilOutProxy;
use pil2_stark::{AirInstanceWitnessComputation, ExecutionCtx, WitnessExecutor, ProofCtx};

use super::{arith::ArithSM, binary::BinarySM, main::MainSM, mem::MemSM};

#[allow(dead_code)]
pub struct ZiskProcessor<F> {
    modules: HashMap<String, Rc<RefCell<dyn AirInstanceWitnessComputation<F>>>>,
    executors: HashMap<String, Rc<RefCell<dyn WitnessExecutor<F>>>>,
}

impl<F: 'static> ZiskProcessor<F> {
    pub fn new(_pilout: &PilOutProxy) -> Self {
        // TODO! Load modules from hints in pilout??

        let mut modules: HashMap<String, Rc<RefCell<dyn AirInstanceWitnessComputation<F>>>> = HashMap::new();
        let mut executors: HashMap<String, Rc<RefCell<dyn WitnessExecutor<F>>>> = HashMap::new();

        let main_module = Rc::new(RefCell::new(MainSM::<F>::new()));
        modules.insert("Main".into(), main_module.clone());
        executors.insert("Main".into(), main_module);

        let arith_module = Rc::new(RefCell::new(ArithSM::<F>::new()));
        modules.insert("Arith".into(), arith_module);

        let binary_module = Rc::new(RefCell::new(BinarySM::<F>::new()));
        modules.insert("Binary".into(), binary_module);

        let mem_module = Rc::new(RefCell::new(MemSM::<F>::new()));
        modules.insert("Memory".into(), mem_module);

        Self { modules, executors }
    }

    pub fn add_module(
        &mut self,
        name: String,
        module: Rc<RefCell<dyn AirInstanceWitnessComputation<F>>>,
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
        executor: Rc<RefCell<dyn WitnessExecutor<F>>>,
    ) -> Result<(), Box<dyn Error>> {
        if self.executors.contains_key(&name) {
            return Err("Executor already exists".into());
        }
        self.executors.insert(name.clone(), executor.clone());
        Ok(())
    }

    pub fn get_modules(&self) -> &HashMap<String, Rc<RefCell<dyn AirInstanceWitnessComputation<F>>>> {
        &self.modules
    }

    pub fn execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("Zisk processor executing");
        for (_, executor) in self.executors.iter() {
            let mut executor = executor.borrow_mut();
            executor.start_execute(proof_ctx, execution_ctx);
            executor.execute(proof_ctx, execution_ctx);
            executor.end_execute(proof_ctx, execution_ctx);
        }
    }
}
