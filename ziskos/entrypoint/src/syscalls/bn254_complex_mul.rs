//! syscall_bn254_complex_mul system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::complex::SyscallComplex256;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBn254ComplexMulParams<'a> {
    pub f1: &'a mut SyscallComplex256,
    pub f2: &'a SyscallComplex256,
}

/// Performs the multiplication of two complex field elements on a complex extension of the Bn254 base field curve,
/// storing the result in the first field element.
///
/// The `Bn254ComplexMul` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Bn254ComplexMul`.
///
/// `Bn254ComplexMul` operates on two field elements, each with two coordinates of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements.
/// The syscall takes as a parameter the address of a structure containing field elements `f1` and `f2`.
/// The result of the addition is stored in `f1`.
///
/// ### Safety
///
/// The caller must ensure that `f1` is a valid pointer to data that is aligned to an eight-byte boundary.
///
/// The caller must ensure that both `f1` and `f2` coordinates are within the range of the BN254 base field.
///
/// The resulting field element will have both coordinates in the range of the BN254 base field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bn254_complex_mul")]
pub extern "C" fn syscall_bn254_complex_mul(
    params: &mut SyscallBn254ComplexMulParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x80A, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let f1 = [params.f1.x, params.f1.y].concat().try_into().unwrap();
        let f2 = [params.f2.x, params.f2.y].concat().try_into().unwrap();
        let mut f3: [u64; 8] = [0; 8];
        precompiles_helpers::bn254_complex_mul(&f1, &f2, &mut f3);
        params.f1.x.copy_from_slice(&f3[0..4]);
        params.f1.y.copy_from_slice(&f3[4..8]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&f3);
        }
    }
}
