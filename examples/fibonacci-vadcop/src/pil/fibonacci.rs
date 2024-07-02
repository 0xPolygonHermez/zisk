use common::{ExecutionCtx, ProofCtx};
use goldilocks::AbstractField;
use wcmanager::{HasSubcomponents, WitnessModule};

use crate::pil::FibonacciVadcopInputs;

use super::helpers::FibonacciTrace;

pub struct Fibonacci<'a, F> {
    subcomponents: Vec<Box<dyn WitnessModule<'a, F> + 'a>>,
}

impl<'a, F> Fibonacci<'a, F> {
    const MY_NAME: &'static str = "Fiboncci";
    pub fn new() -> Self {
        Self { subcomponents: Vec::new() }
    }
}

impl<'a, F: AbstractField> WitnessModule<'a, F> for Fibonacci<'a, F> {
    fn start_proof(&self, public_inputs: &[u8], proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx) {}

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {}

    fn calculate_air_instances_map(&self, proof_ctx: &ProofCtx<F>) {}

    fn calculate_witness(
        &self,
        stage: u32,
        public_inputs: &[u8],
        proof_ctx: &ProofCtx<F>,
        execution_ctx: &ExecutionCtx,
    ) {
        if stage != 1 {
            return;
        }
        
        let mut trace = FibonacciTrace::<F>::new(1 << 10);

        let pi = FibonacciVadcopInputs::from_bytes(public_inputs);
        let m = F::from_canonical_u64(pi[0] as u64);
        trace.a[0] = F::from_canonical_u64(pi[1] as u64);
        trace.b[0] = F::from_canonical_u64(pi[2] as u64);

        for i in 1..1 << 10 {
            trace.a[i] = (trace.a[i - 1].square() + trace.b[i - 1].square()); // % m;
            trace.b[i] = trace.a[i - 1].clone();
        }
    }

    fn name(&self) -> String {
        format!("{}", Self::MY_NAME)
    }
}

impl<'a, F> HasSubcomponents<'a, F> for Fibonacci<'a, F> {
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
