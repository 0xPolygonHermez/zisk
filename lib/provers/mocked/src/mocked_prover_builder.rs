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
    fn build(&self) -> Box<dyn Prover<T>> {
        Box::new(MockedProver::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman::proof_manager::ProverStatus;
    use proofman::ProofCtx;

    // Define a struct for testing purposes
    struct TestData;

    // Implement Prover trait for TestData
    impl Prover<TestData> for TestData {
        // Dummy implementation for testing
        fn build(&self) {}

        fn num_stages(&self) -> u32 {
            1
        }

        fn commit_stage(&mut self, _stage_id: u32, _proof_ctx: &mut ProofCtx<TestData>) -> ProverStatus {
            ProverStatus::OpeningStage
        }
        fn opening_stage(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<TestData>) -> ProverStatus {
            ProverStatus::StagesCompleted
        }
        fn get_commit_stage_root_challenge_256(&self, _stage_id: u32) -> Option<[u64; 4]> {
            unimplemented!()
        }

        fn get_opening_stage_root_challenge_256(&self, _opening_id: u32) -> Option<[u64; 4]> {
            unimplemented!()
        }

        fn add_root_challenge_256_to_transcript(&mut self, _root_challenge: [u64; 4]) {
            unimplemented!()
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
