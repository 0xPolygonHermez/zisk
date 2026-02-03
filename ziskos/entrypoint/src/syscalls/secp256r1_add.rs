//! Secp256r1Add system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::point::SyscallPoint256;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallSecp256r1AddParams<'a> {
    pub p1: &'a mut SyscallPoint256,
    pub p2: &'a SyscallPoint256,
}

/// Performs the addition of two points on the Secp256r1 curve, storing the result in the first point.
///
/// The `Secp256r1Add` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Secp256r1Add`.
///
/// `Secp256r1Add` operates on two points, each with two coordinates of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements.
/// The syscall takes as a parameter the address of a structure containing points `p1` and `p2`.
/// The result of the addition is stored in `p1`.
///
/// ### Safety
///
/// The caller must ensure that `p1` is a valid pointer to data that is aligned to an eight-byte boundary.
///
/// The caller must ensure that both `p1` and `p2` coordinates are within the range of the Secp256r1 base field.
///
/// The resulting point will have both coordinates in the range of the Secp256r1 base field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_secp256r1_add")]
pub extern "C" fn syscall_secp256r1_add(
    params: &mut SyscallSecp256r1AddParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x815, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let p1 = [params.p1.x, params.p1.y].concat().try_into().unwrap();
        let p2 = [params.p2.x, params.p2.y].concat().try_into().unwrap();
        let mut p3: [u64; 8] = [0; 8];
        precompiles_helpers::secp256r1_add(&p1, &p2, &mut p3);
        params.p1.x.copy_from_slice(&p3[0..4]);
        params.p1.y.copy_from_slice(&p3[4..8]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p3);
        }
    }
}
