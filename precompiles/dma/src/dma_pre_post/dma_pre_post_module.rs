use crate::DmaPrePostInput;
use proofman_common::{AirInstance, ProofmanResult};

pub trait DmaPrePostModule<F: Clone>: Send + Sync {
    fn compute_witness(
        &self,
        inputs: &[Vec<DmaPrePostInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>>;
    fn get_name(&self) -> &'static str;
}
