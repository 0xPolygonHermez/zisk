use p3_field::PrimeField;
use proofman_common::{ExecutionCtx, SetupCtx};
use proofman_util::create_buffer_fast;

pub struct WitnessBuffer<F: PrimeField> {
    pub buffer: Vec<F>,
    pub offset: u64,
}

impl<F: PrimeField> WitnessBuffer<F> {
    pub fn new(buffer: Vec<F>, offset: u64) -> Self {
        Self { buffer, offset }
    }
}

pub fn create_prover_buffer<F>(
    ectx: &ExecutionCtx<F>,
    sctx: &SetupCtx<F>,
    airgroup_id: usize,
    air_id: usize,
) -> (Vec<F>, u64) {
    // Compute buffer size using the BufferAllocator
    let (buffer_size, offsets) = ectx
        .buffer_allocator
        .as_ref()
        .get_buffer_info(sctx, airgroup_id, air_id)
        .unwrap_or_else(|err| panic!("Error getting buffer info: {}", err));

    let buffer = create_buffer_fast(buffer_size as usize);

    (buffer, offsets[0])
}
