//! Keccak absorb step (XOR + Keccak-f) system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(not(zisk_guest))]
use tiny_keccak::keccakf;

/// Lanes of the block `syscall_keccak_xor` XORs into the state: the rate of keccak256, 136 bytes.
pub const KECCAK_XOR_LANES: usize = 17;

/// Executes one absorb step of the Keccak sponge: XORs the 17-lane `block` into the first lanes
/// of `state` and applies the Keccak-f\[1600\] permutation to the result, in place.
///
/// Both are passed as two direct operands (no parameter struct); the new state overwrites
/// `state`. It is one instruction for the prover, instead of the 17 XORs and the permutation.
///
/// ### Safety
///
/// The caller must ensure that both pointers are valid and aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_keccak_xor")]
pub unsafe extern "C" fn syscall_keccak_xor(
    state: *mut [u64; 25],
    block: *const [u64; KECCAK_XOR_LANES],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_KECCAK_XOR_ID, a: state, b: block);
    #[cfg(not(zisk_guest))]
    {
        let state = unsafe { &mut *state };
        for (lane, word) in state.iter_mut().zip(unsafe { &*block }) {
            *lane ^= *word;
        }

        // As syscall_keccak_f: the state the permutation runs on is what the cache registers
        crate::zisklib::keccakf_cache::keccakf_cache_on_keccakf(state);

        keccakf(state);

        #[cfg(feature = "hints")]
        if zisk_definitions::KECCAK_RESULTS {
            hints.extend_from_slice(state);
        }
    }
}
