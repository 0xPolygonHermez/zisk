use crate::DmaWithPrePostInput;
use proofman_common::{AirInstance, ProofmanResult};

pub trait DmaWithPrePostModule<F: Clone>: Send + Sync {
    fn compute_witness(
        &self,
        inputs: &[Vec<DmaWithPrePostInput>],
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>>;
    fn get_name(&self) -> &'static str;
}
