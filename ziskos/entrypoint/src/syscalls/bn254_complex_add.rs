//! syscall_bn254_complex_add system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::complex256::SyscallComplex256;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBn254ComplexAddParams<'a> {
    pub f1: &'a mut SyscallComplex256,
    pub f2: &'a SyscallComplex256,
}

/// Performs the addition of two complex field elements on a complex extension of the Bn254 base field curve,
/// storing the result in the first field element.
///
/// The `Bn254ComplexAdd` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Bn254ComplexAdd`.
///
/// `Bn254ComplexAdd` operates on two field elements, each with two coordinates of 256 bits.
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
#[no_mangle]
pub extern "C" fn syscall_bn254_complex_add(params: &mut SyscallBn254ComplexAddParams) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x808, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!()
}
