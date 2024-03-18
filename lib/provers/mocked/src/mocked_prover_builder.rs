use proofman::provers_manager::{Prover, ProverBuilder};
use crate::mocked_prover::MockedProver;

/// MockedProverBuilder struct for use in tests
pub struct MockedProverBuilder<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> MockedProverBuilder<T> {
    pub fn new() -> Self {
        Self { phantom: std::marker::PhantomData }
    }
}

/// ProverBuilder trait implementation for MockedProverBuilder
impl<T: 'static> ProverBuilder<T> for MockedProverBuilder<T> {
    fn build(&mut self) -> Box<dyn Prover<T>> {
        Box::new(MockedProver::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman::proof_manager::ProverStatus;
    use proofman::proof_ctx::ProofCtx;

    // Define a struct for testing purposes
    struct TestData;

    // Implement Prover trait for TestData
    impl Prover<TestData> for TestData {
        // Dummy implementation for testing
        fn build(&mut self) {}
        fn commit_stage(&mut self, _stage_id: u32, _proof_ctx: &mut ProofCtx<TestData>) -> ProverStatus {
            ProverStatus::StagesPending
        }
        fn opening_stage(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<TestData>) -> ProverStatus {
            ProverStatus::StagesCompleted
        }
    }

    #[test]
    fn test_mocked_prover_builder() {
        // Create a MockedProverBuilder instance
        let mut builder = MockedProverBuilder::<TestData>::new();

        // Build a prover using the builder
        let _prover_box = builder.build();
    }
}
