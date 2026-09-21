//! The airs whose stage-1 witness a GPU kernel produces, collected in one place.
//!
//! The executor already knows every state machine, so this is where the prover
//! asks what is device-produced — adding a kernel does not grow the prover
//! backend's dependency list, and the whole set stays readable in one function.
//!
//! Each precompile reports its own declaration and owns its `cfg(gpu)` gate, so
//! this list needs no CUDA-aware build of its own and is simply empty on a
//! build without kernels.
//!
//! The prover reads it before `ProofMan::new`: it sizes the host trace pool and
//! the prefetch zone from it, and refuses the run outright if the environment
//! cannot honour it.

use proofman_common::GpuWitnessAir;

/// Every air this build can produce on the device.
pub fn gpu_witness_airs() -> Vec<GpuWitnessAir> {
    [zisk_precomp_keccakf::gpu_witness_air()].into_iter().flatten().collect()
}
