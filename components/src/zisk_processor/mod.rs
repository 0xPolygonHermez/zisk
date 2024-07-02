use std::{cell::RefCell, collections::HashMap, error::Error, rc::Rc};

use common::{AirInstanceWitnessComputation, ExecutionCtx, ProofCtx};
use goldilocks::AbstractField;
use pilout::pilout_proxy::PilOutProxy;
use wcmanager::WitnessExecutor;

use crate::component::Component;

use super::{arith::ArithSM, binary::BinarySM, main::MainSM, mem::MemorySM};

#[allow(dead_code)]
pub struct ZiskProcessor<'a, F> {
    modules: HashMap<String, Rc<RefCell<dyn AirInstanceWitnessComputation<'a, F>>>>,
    executors: HashMap<String, Rc<RefCell<dyn WitnessExecutor<'a, F>>>>,
    components: HashMap<String, Rc<RefCell<dyn Component<F>>>>,
}

#[allow(dead_code)]
struct BasicProcessorComponent<T> {
    pub id: Option<usize>,
    pub component: Box<dyn Component<T>>,
}

impl<'a, F: AbstractField + 'static> ZiskProcessor<'a, F> {
    pub fn new(_pilout: &PilOutProxy) -> Self {
        // TODO! Load modules from hints in pilout??
        let (modules, executors) = Self::register_modules();

        let components = Self::register_components();

        Self { modules, executors, components }
    }

    fn register_modules() -> (
        HashMap<String, Rc<RefCell<dyn AirInstanceWitnessComputation<'a, F>>>>,
        HashMap<String, Rc<RefCell<dyn WitnessExecutor<'a, F>>>>,
    ) {
        let mut modules: HashMap<String, Rc<RefCell<dyn AirInstanceWitnessComputation<'a, F>>>> = HashMap::new();
        let mut executors: HashMap<String, Rc<RefCell<dyn WitnessExecutor<'a, F>>>> = HashMap::new();

        // let main_module = Rc::new(RefCell::new(MainSM::<F>::new()));
        // modules.insert("Main".into(), main_module.clone());
        // executors.insert("Main".into(), main_module);

        // let arith_module = Rc::new(RefCell::new(ArithSM::<F>::new()));
        // modules.insert("Arith".into(), arith_module);

        // let binary_module = Rc::new(RefCell::new(BinarySM::<F>::new()));
        // modules.insert("Binary".into(), binary_module);

        // let mem_module = Rc::new(RefCell::new(MemorySM::<F>::new()));
        // modules.insert("Memory".into(), mem_module);

        (modules, executors)
    }

    fn register_components() -> HashMap<String, Rc<RefCell<dyn Component<F>>>> {
        let mut components: HashMap<String, Rc<RefCell<dyn Component<F>>>> = HashMap::new();

        // components.insert("mOp".to_string(), Rc::new(RefCell::new(MemorySM::<F>::new())));
        // components.insert("iAdd".to_string(), Rc::new(RefCell::new(MemorySM::<F>::new())));

        components
    }

    pub fn add_module(
        &mut self,
        name: String,
        module: Rc<RefCell<dyn AirInstanceWitnessComputation<'a, F>>>,
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
        executor: Rc<RefCell<dyn WitnessExecutor<'a, F>>>,
    ) -> Result<(), Box<dyn Error>> {
        if self.executors.contains_key(&name) {
            return Err("Executor already exists".into());
        }
        self.executors.insert(name.clone(), executor.clone());
        Ok(())
    }

    pub fn get_modules(&self) -> &HashMap<String, Rc<RefCell<dyn AirInstanceWitnessComputation<'a, F>>>> {
        &self.modules
    }

    pub fn execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        for (_, executor) in self.executors.iter() {
            let mut executor = executor.borrow_mut();
            executor.start_execute(proof_ctx, execution_ctx);
            executor.execute(proof_ctx, execution_ctx);
            executor.end_execute(proof_ctx, execution_ctx);
        }
    }
}
