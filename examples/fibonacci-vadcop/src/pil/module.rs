use common::{ExecutionCtx, ProofCtx, TracePol, WitnessPilOut};
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
    fn start_proof(&self, public_inputs: &[u8], proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx) {
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {}

    fn calculate_air_instances_map(&self, proof_ctx: &ProofCtx<F>) {}

    fn calculate_witness(&self, stage: u32, public_inputs: &[u8], proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        
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
