//! Blake2br system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(not(zisk_guest))]
use zisk_precomp_helpers::blake2b_round;

/// Executes the `Blake2bRound` operation, performing one round of the Blake2b compression function.
///
/// `Blake2bRound` operates on arrays of sixteen `u64` elements. It takes three parameters: `index`, `state`, and
/// `input`. The `index` parameter specifies which round to execute (a number in [0,10)); it is folded into the
/// precompiled instruction as a static argument, so the call site is dispatched over its ten possible values.
/// The `state` parameter is a mutable reference to the current state of the Blake2b compression function, which will be updated in place.
/// The `input` parameter is a reference to the message block being processed.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_blake2b_round")]
pub extern "C" fn syscall_blake2b_round(
    index: u64,
    state: &mut [u64; 16],
    input: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    match index {
        0 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 0)
        }
        1 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 1)
        }
        2 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 2)
        }
        3 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 3)
        }
        4 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 4)
        }
        5 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 5)
        }
        6 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 6)
        }
        7 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 7)
        }
        8 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 8)
        }
        9 => {
            ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: 9)
        }
        _ => panic!("syscall_blake2b_round: invalid round index {index}"),
    }

    #[cfg(not(zisk_guest))]
    {
        blake2b_round(state, input, index as u32);

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(state);
        }
    }
}
