//! Poseidon2 system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use fields::{poseidon2_hash, Goldilocks, Poseidon16, PrimeField64};

/// Executes the Poseidon2 permutation on the given state.
///
/// The `Poseidon2` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Poseidon2`.
///
/// The syscall takes as a parameter the address of a state data (1024 bits = 128 bytes)
/// and the result of the poseidon2 operation is stored at the same location
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_poseidon2")]
pub unsafe extern "C" fn syscall_poseidon2(
    state: *mut [u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x812, state);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        // Get a mutable reference to the state
        let state: &mut [u64; 16] = unsafe { &mut *(state) };

        // Call poseidon2, mapping u64 to Goldilocks elements
        let state_gl = state.map(Goldilocks::new);
        let new_state_gl = poseidon2_hash::<Goldilocks, Poseidon16, 16>(&state_gl);
        for (i, d) in state.iter_mut().enumerate() {
            *d = new_state_gl[i].as_canonical_u64();
        }

        #[cfg(feature = "hints")]
        {
            // For hints, we store the new state in the hints vector
            hints.extend_from_slice(state);
        }
    }
}
