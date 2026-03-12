use crate::DmaInput;
use proofman_common::{AirInstance, ProofmanResult};

pub trait DmaModule<F: Clone>: Send + Sync {
    fn compute_witness(
        &self,
        inputs: &[Vec<DmaInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>>;
    fn get_name(&self) -> &'static str;
}
