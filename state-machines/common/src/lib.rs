mod operations;
mod provable;
mod session;
mod temp;
mod thread_controller;
mod worker;

pub use operations::*;
use proofman_common::{ExecutionCtx, SetupCtx};
use proofman_util::create_buffer_fast;
pub use provable::*;
pub use session::*;
pub use temp::*;
pub use thread_controller::*;
pub use worker::*;

pub fn create_prover_buffer<F>(
    ectx: &ExecutionCtx,
    sctx: &SetupCtx,
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
