//! Blake2br system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use precompiles_helpers::blake2b_round;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBlake2bRoundParams<'a> {
    pub index: u64, // a number in [0,10)
    pub state: &'a mut [u64; 16],
    pub input: &'a [u64; 16],
}

/// Executes the `Blake2bRound` operation, performing one round of the Blake2b compression function.
///
/// `Blake2bRound` operates on arrays of sixteen `u64` elements. The first parameter is a pointer to a structure
/// containing three values: `index`, `state`, and `input`. The `index` parameter specifies which round to execute (a number in [0,10)).
/// The `state` parameter is a mutable reference to the current state of the Blake2b compression function, which will be updated in place.
/// The `input` parameter is a reference to the message block being processed.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_blake2b_round")]
pub extern "C" fn syscall_blake2b_round(
    params: &mut SyscallBlake2bRoundParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, params);

    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        blake2b_round(params.state, params.input, params.index as u32);

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.state);
        }
    }
}
