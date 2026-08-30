//! Blake2sr system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(not(zisk_guest))]
use zisk_precomp_helpers::{blake2s_round, BLAKE2S_ROUNDS};

#[cfg(zisk_guest)]
const BLAKE2S_ROUNDS: usize = 10;

/// Parameters for one BLAKE2s round.
///
/// `state` and `input` are 32-bit words, each occupying its own 64-bit slot with
/// a zero high half. That layout keeps the memory schedule identical to
/// Blake2br's and is what the Blake2sr AIR enforces through its
/// `value: [mem_lo, 0]` memory argument — a slot with anything in its upper half
/// cannot satisfy the constraint.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallBlake2sRoundParams<'a> {
    pub index: u64, // a number in [0,10)
    pub state: &'a mut [u64; 16],
    pub input: &'a [u64; 16],
}

/// Executes the `Blake2sRound` operation, performing one round of the BLAKE2s compression function.
///
/// `Blake2sRound` operates on arrays of sixteen 32-bit values, each held in a `u64` slot with a zero
/// high half. The first parameter is a pointer to a structure containing three values: `index`,
/// `state`, and `input`. The `index` parameter specifies which round to execute (a number in
/// [0,10)). The `state` parameter is a mutable reference to the current state of the BLAKE2s
/// compression function, which will be updated in place. The `input` parameter is a reference to
/// the message block being processed.
///
/// Returns `false` without modifying `state` when `index` is outside `[0, 10)`.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary, and that every word of
/// `state` and `input` fits in 32 bits.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_blake2s_round")]
pub extern "C" fn syscall_blake2s_round(
    params: &mut SyscallBlake2sRoundParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    if params.index >= BLAKE2S_ROUNDS as u64 {
        return false;
    }

    #[cfg(zisk_guest)]
    {
        ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2S_ROUND_ID, params);
        return true;
    }

    #[cfg(not(zisk_guest))]
    {
        let mut v = [0u32; 16];
        let mut m = [0u32; 16];
        for i in 0..16 {
            debug_assert_eq!(params.state[i] >> 32, 0, "blake2s state word is not 32-bit");
            debug_assert_eq!(params.input[i] >> 32, 0, "blake2s input word is not 32-bit");
            v[i] = params.state[i] as u32;
            m[i] = params.input[i] as u32;
        }

        blake2s_round(&mut v, &m, params.index as u32);

        for (slot, word) in params.state.iter_mut().zip(v.iter()) {
            *slot = *word as u64;
        }

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.state);
        }

        true
    }
}

#[cfg(all(test, not(zisk_guest)))]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_wide_round_without_modifying_state() {
        let mut state = [0u64; 16];
        let input = [0u64; 16];
        let mut params =
            SyscallBlake2sRoundParams { index: 1 << 32, state: &mut state, input: &input };

        assert!(!syscall_blake2s_round(
            &mut params,
            #[cfg(feature = "hints")]
            &mut Vec::new(),
        ));
        assert_eq!(state, [0; 16]);
    }
}
