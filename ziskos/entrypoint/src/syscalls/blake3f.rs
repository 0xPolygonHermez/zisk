//! Blake3 system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(not(zisk_guest))]
use zisk_precomp_helpers::blake3_f;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBlake3fParams<'a> {
    pub state: &'a mut [u64; 8],
    pub input: &'a [u64; 8],
}

/// Executes the `Blake3f` operation, performing the Blake3 compression function on the given state and input.
///
/// `Blake3f` operates on arrays of sixteen `u64` elements. The first parameter is a pointer to a structure
/// containing two values: `state` and `input`.
/// The `state` parameter is a mutable reference to the current state of the Blake3 compression function, which will be updated in place.
/// The `input` parameter is a reference to the message block being processed.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_blake3f")]
pub extern "C" fn syscall_blake3f(
    params: &mut SyscallBlake3fParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE3F_ID, params);

    #[cfg(not(zisk_guest))]
    {
        let state_u32: &mut [u32; 16] =
            unsafe { &mut *(params.state.as_mut_ptr() as *mut [u32; 16]) };
        let input_u32: &[u32; 16] = unsafe { &*(params.input.as_ptr() as *const [u32; 16]) };
        blake3_f(state_u32, input_u32);

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.state);
        }
    }
}
