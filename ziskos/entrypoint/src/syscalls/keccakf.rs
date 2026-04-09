//! Keccak system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use tiny_keccak::keccakf;

/// Executes the Keccak256 permutation on the given state.
///
/// The Keccak256 permutation operates on an array of twenty-five `u64` elements, which represents the internal state of the Keccak algorithm.
/// The input state is modified in place to produce the output.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_keccak_f")]
pub unsafe extern "C" fn syscall_keccak_f(
    state: *mut [u64; 25],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(zisk_definitions::SYSCALL_KECCAKF_ID, state);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        // Call keccakf
        keccakf(unsafe { &mut *state });

        // Store results in hints vector
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(unsafe { &*state });
        }
    }
}
