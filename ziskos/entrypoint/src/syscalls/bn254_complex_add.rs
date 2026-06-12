//! syscall_bn254_complex_add system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::complex::SyscallComplex256;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBn254ComplexAddParams<'a> {
    pub f1: &'a mut SyscallComplex256,
    pub f2: &'a SyscallComplex256,
}

/// Performs the addition of two complex field elements on a complex extension of the BN254 base field curve,
/// storing the result in the first field element.
///
/// `Bn254ComplexAdd` operates on two field elements, each with two coordinates of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements.
/// The syscall takes as a parameter the address of a structure containing field elements `f1` and `f2`.
/// The result of the addition is stored in `f1`.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The `f1` and `f2` coordinates are not required to be canonical representatives of elements
/// in the BN254 base field.
///
/// The resulting field element will have both coordinates reduced to canonical representatives
/// in the range of the BN254 base field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bn254_complex_add")]
pub extern "C" fn syscall_bn254_complex_add(
    params: &mut SyscallBn254ComplexAddParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BN254_COMPLEX_ADD_ID, params);
    #[cfg(not(zisk_guest))]
    {
        let f1 = [params.f1.x, params.f1.y].concat().try_into().unwrap();
        let f2 = [params.f2.x, params.f2.y].concat().try_into().unwrap();
        let mut f3: [u64; 8] = [0; 8];
        precompiles_helpers::bn254_complex_add(&f1, &f2, &mut f3);
        params.f1.x.copy_from_slice(&f3[0..4]);
        params.f1.y.copy_from_slice(&f3[4..8]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&f3);
        }
    }
}
