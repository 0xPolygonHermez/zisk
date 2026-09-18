//! FFI to the CUDA Keccakf witness kernel (`cu/keccakf_witness.cu`).
//!
//! Spike scaffolding: the kernel produces the same packed trace the CPU path
//! produces, so the two can be compared byte for byte and timed against each
//! other. Nothing in the production witness path calls this yet.

use super::KeccakfInput;

/// One operation as the kernel consumes it. Mirrors `KeccakfGpuOp` in the `.cu`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GpuOp {
    pub state: [u64; 25],
    pub step: u64,
    pub addr: u64,
}

impl From<&KeccakfInput> for GpuOp {
    fn from(input: &KeccakfInput) -> Self {
        Self { state: input.state, step: input.step_main, addr: input.addr_main as u64 }
    }
}

/// Timings reported by one `run` call, all in milliseconds.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct GpuTimings {
    pub alloc_ms: f64,
    pub h2d_ms: f64,
    pub kernel_ms: f64,
    pub kernel_avg_ms: f64,
    pub d2h_ms: f64,
    pub out_bytes: u64,
}

extern "C" {
    fn keccakf_gpu_available() -> i32;
    fn keccakf_gpu_row_words() -> i32;
    fn keccakf_gpu_clocks() -> i32;
    fn keccakf_gpu_witness(
        ops: *const GpuOp,
        num_ops: u32,
        out_host: *mut u64,
        iters: u32,
        block: i32,
        timings: *mut GpuTimings,
    ) -> i32;
}

pub fn available() -> bool {
    unsafe { keccakf_gpu_available() == 1 }
}

/// Words per packed trace row, as the kernel lays them out.
pub fn row_words() -> usize {
    unsafe { keccakf_gpu_row_words() as usize }
}

/// Rows per slot, as the kernel lays them out.
pub fn clocks() -> usize {
    unsafe { keccakf_gpu_clocks() as usize }
}

/// Number of u64 the packed trace of `num_ops` operations occupies.
pub fn out_words(num_ops: usize) -> usize {
    num_ops.div_ceil(2) * clocks() * row_words()
}

/// Runs the kernel `iters` times over `ops`.
///
/// `out` receives the packed trace when `Some`; passing `None` skips the
/// device-to-host copy, which is the configuration that matters if the trace is
/// to be consumed on the device.
pub fn run(
    ops: &[GpuOp],
    out: Option<&mut [u64]>,
    iters: u32,
    block: i32,
) -> Result<GpuTimings, i32> {
    let mut timings = GpuTimings::default();
    let out_ptr = match out {
        Some(buffer) => {
            assert_eq!(buffer.len(), out_words(ops.len()), "output buffer has the wrong length");
            buffer.as_mut_ptr()
        }
        None => std::ptr::null_mut(),
    };
    let rc = unsafe {
        keccakf_gpu_witness(
            ops.as_ptr(),
            ops.len() as u32,
            out_ptr,
            iters,
            block,
            &mut timings as *mut GpuTimings,
        )
    };
    if rc == 0 {
        Ok(timings)
    } else {
        Err(rc)
    }
}
