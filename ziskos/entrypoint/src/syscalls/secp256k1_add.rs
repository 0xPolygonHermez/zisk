//! Secp256k1Add system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::point256::SyscallPoint256;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallSecp256k1AddParams<'a> {
    pub p1: &'a mut SyscallPoint256,
    pub p2: &'a SyscallPoint256,
}

/// Performs the addition of two points on the Secp256k1 curve, storing the result in the first point.
///
/// The `Secp256k1Add` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Secp256k1Add`.
///
/// `Secp256k1Add` operates on two points, each with two coordinates of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements.
/// The syscall takes as a parameter the address of a structure containing points `p1` and `p2`.
/// The result of the addition is stored in `p1`.
///
/// ### Safety
///
/// The caller must ensure that `p1` is a valid pointer to data that is aligned to an eight-byte boundary.
///
/// The caller must ensure that both `p1` and `p2` coordinates are within the range of the Secp256k1 base field.
///
/// The resulting point will have both coordinates in the range of the Secp256k1 base field.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_secp256k1_add(params: &mut SyscallSecp256k1AddParams) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x803, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!()
}
