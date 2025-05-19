//! syscall_bn254_curve_add system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::point256::SyscallPoint256;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBn254CurveAddParams<'a> {
    pub p1: &'a mut SyscallPoint256,
    pub p2: &'a SyscallPoint256,
}

/// Performs the addition of two points on the Bn254 curve, storing the result in the first point.
///
/// The `Bn254CurveAdd` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Bn254CurveAdd`.
///
/// `Bn254CurveAdd` operates on two points, each with two coordinates of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements.
/// The syscall takes as a parameter the address of a structure containing points `p1` and `p2`.
/// The result of the addition is stored in `p1`.
///
/// ### Safety
///
/// The caller must ensure that `p1` is a valid pointer to data that is aligned to an eight-byte boundary.
///
/// The caller must ensure that both `p1` and `p2` coordinates are within the range of the BN254 base field.
///
/// The resulting point will have both coordinates in the range of the BN254 base field.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bn254_curve_add(params: &mut SyscallBn254CurveAddParams) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x806, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!()
}
