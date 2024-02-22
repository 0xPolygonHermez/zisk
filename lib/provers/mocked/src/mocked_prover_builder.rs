use proofman::provers_manager::{Prover, ProverBuilder};
use crate::mocked_prover::MockedProver;

pub struct MockedProverBuilder<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> MockedProverBuilder<T> {
    pub fn new() -> Self {
        Self { phantom: std::marker::PhantomData }
    }
}

impl<T: 'static> ProverBuilder<T> for MockedProverBuilder<T> {
    fn build(&mut self) -> Box<dyn Prover<T>> {
        Box::new(MockedProver::new())
    }
}
