//! Compiles the GPU Keccakf witness kernel when CUDA is available.
//!
//! Sets `cfg(gpu)`; everything GPU-side in this crate is gated on it, so the
//! crate builds unchanged on hosts without nvcc.

fn main() {
    zisk_cuda_build::compile(&zisk_cuda_build::CudaLib {
        name: "keccakf_cu",
        dir: "cu",
        sources: &["keccakf_witness.cu"],
        // Kernel work here is profiled under Nsight often enough to be worth
        // the line tables; they do not affect codegen.
        extra_nvcc_flags: "-lineinfo",
        ..Default::default()
    });
}
