use common::{ExecutionCtx, ProofCtx};
use goldilocks::AbstractField;
use wcmanager::{HasSubcomponents, WitnessModule};

use crate::pil::FibonacciVadcopInputs;

use super::{get_fibonacci_vadcop_pilout, helpers::FibonacciTrace};

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
    fn start_proof(&self, _public_inputs: &[u8], _proof_ctx: &mut ProofCtx<F>, _execution_ctx: &ExecutionCtx) {}

    fn end_proof(&self, _proof_ctx: &ProofCtx<F>) {}

    fn calculate_air_instances_map(&self, _proof_ctx: &ProofCtx<F>) {}

    fn calculate_witness(
        &self,
        stage: u32,
        public_inputs: &[u8],
        _proof_ctx: &ProofCtx<F>,
        _execution_ctx: &ExecutionCtx,
    ) {
        if stage != 1 {
            return;
        }

        let pilout = get_fibonacci_vadcop_pilout();
        let air = pilout.get_air("AirGroup_1", "FibonacciSquare").unwrap_or_else(|| panic!("Air not found"));
        let num_rows: usize = 1 << air.num_rows;

        let mut trace = FibonacciTrace::<F>::new(num_rows);

        let pi = FibonacciVadcopInputs::from_bytes(public_inputs);

        trace.a[0] = F::from_canonical_u64(pi.a as u64);
        trace.b[0] = F::from_canonical_u64(pi.b as u64);

        for i in 1..num_rows {
            trace.a[i] = trace.a[i - 1].square() + trace.b[i - 1].square();
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
