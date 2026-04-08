//! Blake3 system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use precompiles_helpers::blake3_f;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBlake3fParams<'a> {
    pub state: &'a mut [u64; 8],
    pub input: &'a [u64; 8],
}

#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_blake3f")]
pub extern "C" fn syscall_blake3f(
    params: &mut SyscallBlake3fParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE3F_ID, params);

    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
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
