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
/// The `Sha256` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Sha256`.
///
/// The syscall takes as a parameter the address of a state data (256 bits = 32 bytes)
/// and the address of an input data (512 bits = 64 bytes), and the result of the
/// sha256f operation (256 bits = 32 bytes) is stored at the same location as the state.
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
    ziskos_syscall!(0x805, params);
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
