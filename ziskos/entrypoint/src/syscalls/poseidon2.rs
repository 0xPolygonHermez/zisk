//! Poseidon2 system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(not(zisk_guest))]
use fields::{poseidon2_hash, Goldilocks, Poseidon2_16, PrimeField64};

/// Executes the Poseidon2 permutation on the given state.
///
/// The Poseidon2 permutation operates on an array of sixteen `u64` elements, which represents the internal state of the Poseidon2 algorithm.
/// The input state is modified in place to produce the output.
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
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_POSEIDON2_ID, state);
    #[cfg(not(zisk_guest))]
    {
        // Get a mutable reference to the state
        let state: &mut [u64; 16] = unsafe { &mut *(state) };

        // Call poseidon2, mapping u64 to Goldilocks elements
        let state_gl = state.map(Goldilocks::new);
        let new_state_gl = poseidon2_hash::<Goldilocks, Poseidon2_16, 16>(&state_gl);
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
