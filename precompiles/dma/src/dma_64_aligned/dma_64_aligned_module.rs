use crate::Dma64AlignedInput;
use proofman_common::{AirInstance, ProofmanResult};
use zisk_common::SegmentId;

pub trait Dma64AlignedModule<F: Clone>: Send + Sync {
    fn compute_witness(
        &self,
        inputs: &[Vec<Dma64AlignedInput>],
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>>;
    fn get_name(&self) -> &'static str;
}
