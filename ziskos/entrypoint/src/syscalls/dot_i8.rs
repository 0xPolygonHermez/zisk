//! Dot-product system call interception (experimental)

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall_ret_u64;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallDotI8Params {
    /// Address of the first vector, 8-byte aligned.
    pub x: *const i8,
    /// Address of the second vector, 8-byte aligned.
    pub y: *const i8,
    /// Length of both vectors in bytes, a multiple of 8.
    pub n: u64,
}

/// Executes the experimental `DotI8` operation: `Σ x[i]·y[i]` over `n` signed bytes, as a
/// wrapping `i32` sum.
///
/// **Experimental:** ZisK's emulator runs it, but no state machine proves it yet, so a program
/// that calls it cannot be proven.
///
/// ### Safety
///
/// `x` and `y` must point to `n` readable bytes, be 8-byte aligned, and `n` must be a multiple of 8.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_dot_i8")]
pub unsafe extern "C" fn syscall_dot_i8(params: &SyscallDotI8Params) -> i32 {
    #[cfg(not(zisk_guest))]
    {
        let n = params.n as usize;
        let x = unsafe { core::slice::from_raw_parts(params.x, n) };
        let y = unsafe { core::slice::from_raw_parts(params.y, n) };
        x.iter().zip(y).fold(0i32, |acc, (&a, &b)| acc.wrapping_add(a as i32 * b as i32))
    }
    #[cfg(zisk_guest)]
    {
        ziskos_syscall_ret_u64!(zisk_definitions::SYSCALL_DOT_I8_ID, params) as i32
    }
}
