//! FFI to the CUDA Keccakf witness kernel (`cu/keccakf_witness.cu`).
//!
//! The kernel's entry points are generated C-side by `ZISK_GPU_WITNESS_ENTRIES`
//! and declared here by [`zisk_gpu_witness::declare_kernel!`], so all this file
//! carries is what is genuinely Keccakf's: the operation layout, and the
//! benchmark driver that exists only to time the kernel against the CPU fill.

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

zisk_gpu_witness::declare_kernel! {
    /// Keccakf packed-witness kernel: one permutation per warp lane.
    pub KeccakfKernel, prefix = keccakf, op = GpuOp,
}

/// Rows per slot, as the kernel lays them out.
pub fn clocks() -> usize {
    unsafe { keccakf_gpu_clocks() as usize }
}

/// Timings reported by one [`run`] call, all in milliseconds.
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

/// Benchmark driver: runs the kernel `iters` times over `ops`, allocating and
/// copying on its own.
///
/// Only the benchmark uses this. Correctness goes through
/// [`GpuWitnessKernel::fill_probe`](zisk_gpu_witness::GpuWitnessKernel::fill_probe),
/// which drives the entry the prover calls.
///
/// `out` receives the packed trace when `Some`; passing `None` skips the
/// device-to-host copy, which is the configuration that matters when the trace
/// is consumed on the device.
pub fn run(
    ops: &[GpuOp],
    out: Option<&mut [u64]>,
    iters: u32,
    block: i32,
) -> Result<GpuTimings, i32> {
    use zisk_gpu_witness::GpuWitnessKernel;
    let mut timings = GpuTimings::default();
    let out_ptr = match out {
        Some(buffer) => {
            assert_eq!(
                buffer.len(),
                KeccakfKernel::out_words(ops.len()),
                "output buffer has the wrong length"
            );
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
