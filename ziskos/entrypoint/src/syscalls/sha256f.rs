//! Sha256 system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use sha2::compress256;

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[allow(deprecated)]
use sha2::digest::generic_array::{typenum::U64, GenericArray};

#[derive(Debug)]
#[repr(C)]
pub struct SyscallSha256Params<'a> {
    pub state: &'a mut [u64; 4],
    pub input: &'a [u64; 8],
}

/// Executes the SHA-256 extend and compress function on the given state and input.
///
/// The SHA-256 compression function operates on a state of four `u64` elements (representing the internal state of the hash function)
/// and an input of eight `u64` elements (representing a 512-bit message block).
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_sha256_f")]
pub extern "C" fn syscall_sha256_f(
    params: &mut SyscallSha256Params,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(zisk_definitions::SYSCALL_SHA256F_ID, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        sha256f(params.state, params.input);

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.state);
        }
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[allow(deprecated)]
fn sha256f(state: &mut [u64; 4], input: &[u64; 8]) {
    let state_u32: &mut [u32; 8] = unsafe { &mut *(state.as_mut_ptr() as *mut [u32; 8]) };
    let input_u8: &[GenericArray<u8, U64>; 1] =
        unsafe { &*(input.as_ptr() as *const [GenericArray<u8, U64>; 1]) };
    compress256(state_u32, input_u8);
}
