//! Blake2s system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(not(zisk_guest))]
use zisk_precomp_helpers::blake2s_f;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBlake2sfParams<'a> {
    pub state: &'a mut [u64; 8],
    pub input: &'a [u64; 8],
}

/// Executes the `Blake2sf` operation, performing the 10-round Blake2s permutation (the compression
/// function without initialisation and feed-forward) on the given state and input.
///
/// `Blake2sf` operates on arrays of eight `u64` elements, each holding two little-endian `u32` words. The first parameter is a pointer to a structure
/// containing two values: `state` and `input`.
/// The `state` parameter is a mutable reference to the working vector v of the Blake2s compression function, which will be updated in place.
/// The `input` parameter is a reference to the message block being processed.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_blake2sf")]
pub extern "C" fn syscall_blake2sf(
    params: &mut SyscallBlake2sfParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2SF_ID, params);

    #[cfg(not(zisk_guest))]
    {
        let state_u32: &mut [u32; 16] =
            unsafe { &mut *(params.state.as_mut_ptr() as *mut [u32; 16]) };
        let input_u32: &[u32; 16] = unsafe { &*(params.input.as_ptr() as *const [u32; 16]) };
        blake2s_f(state_u32, input_u32);

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.state);
        }
    }
}
