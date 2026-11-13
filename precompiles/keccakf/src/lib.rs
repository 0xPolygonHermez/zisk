mod keccakf;
mod keccakf_chi_table;
mod keccakf_constants;
mod keccakf_family;
#[cfg(gpu)]
pub mod keccakf_gpu;
mod keccakf_mem_inputs;
mod keccakf_proxy;
mod keccakf_xor5_table;

pub use keccakf::*;
pub use keccakf_family::*;
pub use keccakf_proxy::*;

/// Operations one full Keccakf air instance can hold. The GPU witness declaration
/// sizes its staging from this: it is the most ops a single instance can stage.
pub const MAX_OPS_PER_INSTANCE: usize = KECCAKF_OPS_PER_INSTANCE;

/// This crate's GPU witness declaration, or `None` when the build has no kernel.
///
/// The `cfg(gpu)` gate lives here, in the crate that owns the kernel, so callers
/// collecting declarations never need a CUDA-aware build of their own.
pub fn gpu_witness_air() -> Option<::proofman_common::GpuWitnessAir> {
    #[cfg(gpu)]
    {
        use ::proofman_common::{GpuWitnessAir, TraceLayout};
        use keccakf_gpu::KeccakfKernel;
        Some(GpuWitnessAir::new(
            ::zisk_pil::KeccakfTrace::<()>::AIRGROUP_ID,
            ::zisk_pil::KeccakfTrace::<()>::AIR_ID,
            // Worst case for one instance, which is what the staging is sized against.
            MAX_OPS_PER_INSTANCE as u64 * KeccakfKernel::BYTES_PER_OP,
            KeccakfKernel::BYTES_PER_OP,
            TraceLayout::PackedCm1,
            KeccakfKernel::FILL,
        ))
    }
    #[cfg(not(gpu))]
    {
        None
    }
}

use keccakf_chi_table::*;
use keccakf_constants::*;
pub use keccakf_xor5_table::*;

#[cfg(test)]
mod keccakf_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::{ELF_KECCAK, ELF_KECCAKF_CACHE};

    /// Number of `syscall_keccak_f` invocations the guest will perform.
    const NUM_KECCAKFS: u64 = 10;

    #[test]
    fn keccakf_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_KECCAKFS);

        ELF_KECCAK.run_emulation(stdin, None).expect("keccak guest emulation failed");
    }

    /// Drives the `fcall_set_keccakf_cache_index` / `fcall_get_keccakf_cache_index` pair from a
    /// guest: the guest asserts every hit, miss and registration lifetime itself, so a failure
    /// surfaces as a failed emulation.
    #[test]
    fn keccakf_cache_tests() {
        ELF_KECCAKF_CACHE
            .run_emulation(ZiskStdin::new(), None)
            .expect("keccakf cache guest emulation failed");
    }
}
