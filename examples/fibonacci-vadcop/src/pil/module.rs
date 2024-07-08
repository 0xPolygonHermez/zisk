use common::{ExecutionCtx, ProofCtx};
use wcmanager::{HasSubcomponents, WitnessModule};

pub struct Module<'a, F> {
    subcomponents: Vec<Box<dyn WitnessModule<'a, F> + 'a>>,
}

impl<'a, F> Module<'a, F> {
    const MY_NAME: &'static str = "Module  ";
    pub fn new() -> Self {
        Self { subcomponents: Vec::new() }
    }
}

impl<'a, F> WitnessModule<'a, F> for Module<'a, F> {
    fn start_proof(&self, _public_inputs: &[u8], _proof_ctx: &mut ProofCtx<F>, _execution_ctx: &ExecutionCtx) {
    }

    fn end_proof(&self, _proof_ctx: &ProofCtx<F>) {}

    fn calculate_air_instances_map(&self, _proof_ctx: &ProofCtx<F>) {}

    fn calculate_witness(&self, _stage: u32, _public_inputs: &[u8], _proof_ctx: &ProofCtx<F>, _execution_ctx: &ExecutionCtx) {

    }

    fn name(&self) -> String {
        format!("{}", Self::MY_NAME)
    }
}

impl<'a, F> HasSubcomponents<'a, F> for Module<'a, F> {
    fn add_subcomponent(&mut self, subcomponent: Box<dyn WitnessModule<'a, F> + 'a>) {
        self.subcomponents.push(subcomponent);
    }

    fn get_subcomponents(&self) -> &[Box<dyn WitnessModule<'a, F> + 'a>] {
        &self.subcomponents
    }

    fn get_subcomponents_mut(&mut self) -> &mut Vec<Box<dyn WitnessModule<'a, F> + 'a>> {
        &mut self.subcomponents
    }
}
