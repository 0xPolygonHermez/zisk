//! The airs whose stage-1 witness a GPU kernel produces. Each precompile owns its
//! `cfg(gpu)` gate, so the list is empty on a build without kernels.

use proofman_common::GpuWitnessAir;

/// Every air this build can produce on the device.
pub fn gpu_witness_airs() -> Vec<GpuWitnessAir> {
    [zisk_precomp_keccakf::gpu_witness_air()].into_iter().flatten().collect()
}
