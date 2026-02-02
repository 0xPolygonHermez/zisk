//! syscall_bls12_381_complex_sub system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::complex::SyscallComplex384;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBls12_381ComplexSubParams<'a> {
    pub f1: &'a mut SyscallComplex384,
    pub f2: &'a SyscallComplex384,
}

/// Performs the subtraction of two complex field elements on a complex extension of the Bls12_381 base field curve,
/// storing the result in the first field element.
///
/// The `Bls12_381ComplexSub` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Bls12_381ComplexSub`.
///
/// `Bls12_381ComplexSub` operates on two field elements, each with two coordinates of 384 bits.
/// Each coordinate is represented as an array of six `u64` elements.
/// The syscall takes as a parameter the address of a structure containing field elements `f1` and `f2`.
/// The result of the addition is stored in `f1`.
///
/// ### Safety
///
/// The caller must ensure that `f1` is a valid pointer to data that is aligned to an eight-byte boundary.
///
/// The caller must ensure that both `f1` and `f2` coordinates are within the range of the BLS12-381 base field.
///
/// The resulting field element will have both coordinates in the range of the BLS12-381 base field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bls12_381_complex_sub")]
pub extern "C" fn syscall_bls12_381_complex_sub(
    params: &mut SyscallBls12_381ComplexSubParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x80F, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let f1 = [params.f1.x, params.f1.y].concat().try_into().unwrap();
        let f2 = [params.f2.x, params.f2.y].concat().try_into().unwrap();
        let mut f3: [u64; 12] = [0; 12];
        precompiles_helpers::bls12_381_complex_sub(&f1, &f2, &mut f3);
        params.f1.x.copy_from_slice(&f3[0..6]);
        params.f1.y.copy_from_slice(&f3[6..12]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&f3);
        }
    }
}
