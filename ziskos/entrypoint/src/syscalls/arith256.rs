//! Arith256 system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallArith256Params<'a> {
    pub a: &'a [u64; 4],
    pub b: &'a [u64; 4],
    pub c: &'a [u64; 4],
    pub dl: &'a mut [u64; 4],
    pub dh: &'a mut [u64; 4],
}

/// Executes the `Arith256` operation, performing a 256-bit multiplication and addition:
/// `a * b + c = dh | dl`.
///
/// `Arith256` operates on arrays of four `u64` elements. The first parameter is a pointer to a structure
/// containing five values: `a`, `b`, `c`, and the result, two 256-bit chunks for `d`:
/// - `dh`: The most significant 256-bit chunk.
/// - `dl`: The least significant 256-bit chunk.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_arith256")]
pub extern "C" fn syscall_arith256(
    params: &mut SyscallArith256Params,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_ARITH256_ID, params);
    #[cfg(not(zisk_guest))]
    {
        precompiles_helpers::arith256(params.a, params.b, params.c, params.dl, params.dh);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.dl);
            hints.extend_from_slice(params.dh);
        }
    }
}
