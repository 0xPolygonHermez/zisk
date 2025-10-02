//! syscall_bls12_381_curve_add system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::point::SyscallPoint384;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBls12_381CurveAddParams<'a> {
    pub p1: &'a mut SyscallPoint384,
    pub p2: &'a SyscallPoint384,
}

/// Performs the addition of two points on the Bls12_381 curve, storing the result in the first point.
///
/// The `Bls12_381CurveAdd` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Bls12_381CurveAdd`.
///
/// `Bls12_381CurveAdd` operates on two points, each with two coordinates of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements.
/// The syscall takes as a parameter the address of a structure containing points `p1` and `p2`.
/// The result of the addition is stored in `p1`.
///
/// ### Safety
///
/// The caller must ensure that `p1` is a valid pointer to data that is aligned to an eight-byte boundary.
///
/// The caller must ensure that both `p1` and `p2` coordinates are within the range of the BLS12-381 base field.
///
/// The resulting point will have both coordinates in the range of the BLS12-381 base field.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12_381_curve_add(params: &mut SyscallBls12_381CurveAddParams) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x80C, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!()
}
