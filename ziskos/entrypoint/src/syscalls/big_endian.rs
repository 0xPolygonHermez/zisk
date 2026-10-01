//! Big-endian twins of the ArithEq / ArithEq384 syscalls.
//!
//! Each `syscall_*_be` performs the same operation as its little-endian twin, but every 256-bit
//! (or 384-bit) operand is stored in memory as a **big-endian integer**: its most significant byte
//! at the lowest address, the way EVM words and most serialized field elements come. The parameter
//! structures are the same ones the little-endian syscalls take; what changes is how the bytes of
//! each `[u64; 4]` / `[u64; 6]` are read: as a 32- or 48-byte big-endian number rather than as
//! little-endian limbs. A point or a complex element is two such numbers, one per coordinate.
//!
//! Inputs and results are big-endian alike, so a guest never converts: the precompile does it in
//! the VM (the `big_endian: 1` airs prove the `OP_*_BE` opcodes). Everything else said about the
//! little-endian syscall (alignment, canonical inputs, what is written where) applies unchanged.
//!
//! [`swap_endianness`] converts between the two representations when a guest does need to.

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

use super::{
    SyscallArith256ModParams, SyscallArith256Params, SyscallArith384ModParams,
    SyscallBls12_381ComplexAddParams, SyscallBls12_381ComplexMulParams,
    SyscallBls12_381ComplexSubParams, SyscallBls12_381CurveAddParams, SyscallBn254ComplexAddParams,
    SyscallBn254ComplexMulParams, SyscallBn254ComplexSubParams, SyscallBn254CurveAddParams,
    SyscallPoint256, SyscallPoint384, SyscallSecp256k1AddParams, SyscallSecp256r1AddParams,
};

/// Converts, in place, the `[u64; N]` memory image of a big-endian integer into its little-endian
/// limbs, and back (it is an involution): reverse the words, then swap the bytes of each word.
///
/// Reading a big-endian 256-bit number as four little-endian `u64` gives the limbs in reverse order
/// with the bytes of every limb reversed; this undoes both.
#[inline]
pub fn swap_endianness(words: &mut [u64]) {
    words.reverse();
    for w in words.iter_mut() {
        *w = w.swap_bytes();
    }
}

/// [`swap_endianness`] applied to each of the `limbs`-word integers `words` is made of (a point is
/// two coordinates, a complex element two components, each a separate big-endian integer).
#[inline]
pub fn swap_endianness_elements(words: &mut [u64], limbs: usize) {
    debug_assert_eq!(words.len() % limbs, 0);
    for element in words.chunks_exact_mut(limbs) {
        swap_endianness(element);
    }
}

/// `value` converted to the other endianness, as a new array (see [`swap_endianness`]).
#[cfg(not(zisk_guest))]
#[inline]
fn swapped<const N: usize>(value: &[u64; N]) -> [u64; N] {
    let mut out = *value;
    swap_endianness(&mut out);
    out
}

/// Big-endian twin of [`super::syscall_arith256`]: `a * b + c = dh | dl`, with `a`, `b`, `c`, `dl`
/// and `dh` stored as big-endian 256-bit integers.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_arith256_be")]
pub extern "C" fn syscall_arith256_be(
    params: &mut SyscallArith256Params,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_ARITH256_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        let (a, b, c) = (swapped(params.a), swapped(params.b), swapped(params.c));
        let mut dl = [0u64; 4];
        let mut dh = [0u64; 4];
        zisk_precomp_helpers::arith256(&a, &b, &c, &mut dl, &mut dh);
        *params.dl = swapped(&dl);
        *params.dh = swapped(&dh);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.dl);
            hints.extend_from_slice(params.dh);
        }
    }
}

/// Big-endian twin of [`super::syscall_arith256_mod`]: `d = (a * b + c) mod module`, every operand
/// a big-endian 256-bit integer.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_arith256_mod_be")]
pub extern "C" fn syscall_arith256_mod_be(
    params: &mut SyscallArith256ModParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_ARITH256_MOD_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        let (a, b, c, module) =
            (swapped(params.a), swapped(params.b), swapped(params.c), swapped(params.module));
        let mut d = [0u64; 4];
        zisk_precomp_helpers::arith256_mod(&a, &b, &c, &module, &mut d);
        *params.d = swapped(&d);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.d);
        }
    }
}

/// Big-endian twin of [`super::syscall_arith384_mod`]: `d = (a * b + c) mod module`, every operand
/// a big-endian 384-bit integer.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_arith384_mod_be")]
pub extern "C" fn syscall_arith384_mod_be(
    params: &mut SyscallArith384ModParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_ARITH384_MOD_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        let (a, b, c, module) =
            (swapped(params.a), swapped(params.b), swapped(params.c), swapped(params.module));
        let mut d = [0u64; 6];
        zisk_precomp_helpers::arith384_mod(&a, &b, &c, &module, &mut d);
        *params.d = swapped(&d);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.d);
        }
    }
}

/// Native fallback shared by the point / complex syscalls: `p1 <- op(p1, p2)` (or `op(p1)` when
/// `p2` is `None`), with each coordinate a big-endian integer of `N` words.
#[cfg(not(zisk_guest))]
#[inline]
fn point_op_be<const N: usize, const N2: usize>(
    x1: &mut [u64; N],
    y1: &mut [u64; N],
    p2: Option<(&[u64; N], &[u64; N])>,
    op: impl FnOnce(&[u64; N2], Option<&[u64; N2]>, &mut [u64; N2]),
) -> [u64; N2] {
    let mut p1 = [0u64; N2];
    p1[..N].copy_from_slice(&swapped(x1));
    p1[N..].copy_from_slice(&swapped(y1));
    let p2 = p2.map(|(x2, y2)| {
        let mut p = [0u64; N2];
        p[..N].copy_from_slice(&swapped(x2));
        p[N..].copy_from_slice(&swapped(y2));
        p
    });
    let mut p3 = [0u64; N2];
    op(&p1, p2.as_ref(), &mut p3);
    swap_endianness_elements(&mut p3, N);
    x1.copy_from_slice(&p3[..N]);
    y1.copy_from_slice(&p3[N..]);
    p3
}

macro_rules! be_point_add_syscall {
    ($(#[$doc:meta])* $name:ident, $hints_name:literal, $id:ident, $params:ident, $p1:ident, $p2:ident, $helper:ident, $n:literal, $n2:literal) => {
        $(#[$doc])*
        #[allow(unused_variables)]
        #[cfg_attr(not(feature = "hints"), no_mangle)]
        #[cfg_attr(feature = "hints", export_name = $hints_name)]
        pub extern "C" fn $name(
            params: &mut $params,
            #[cfg(feature = "hints")] hints: &mut Vec<u64>,
        ) {
            #[cfg(zisk_guest)]
            ziskos_syscall!(zisk_definitions::$id, params);
            #[cfg(not(zisk_guest))]
            {
                let p3 = point_op_be::<$n, $n2>(
                    &mut params.$p1.x,
                    &mut params.$p1.y,
                    Some((&params.$p2.x, &params.$p2.y)),
                    |p1, p2, p3| zisk_precomp_helpers::$helper(p1, p2.unwrap(), p3),
                );
                #[cfg(feature = "hints")]
                {
                    hints.extend_from_slice(&p3);
                }
            }
        }
    };
}

macro_rules! be_point_dbl_syscall {
    ($(#[$doc:meta])* $name:ident, $hints_name:literal, $id:ident, $point:ident, $helper:ident, $n:literal, $n2:literal) => {
        $(#[$doc])*
        #[allow(unused_variables)]
        #[cfg_attr(not(feature = "hints"), no_mangle)]
        #[cfg_attr(feature = "hints", export_name = $hints_name)]
        pub extern "C" fn $name(p: &mut $point, #[cfg(feature = "hints")] hints: &mut Vec<u64>) {
            #[cfg(zisk_guest)]
            ziskos_syscall!(zisk_definitions::$id, p);
            #[cfg(not(zisk_guest))]
            {
                let p3 = point_op_be::<$n, $n2>(&mut p.x, &mut p.y, None, |p1, _, p3| {
                    zisk_precomp_helpers::$helper(p1, p3)
                });
                #[cfg(feature = "hints")]
                {
                    hints.extend_from_slice(&p3);
                }
            }
        }
    };
}

be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_secp256k1_add`]: `p1 <- p1 + p2` on secp256k1, every
    /// coordinate a big-endian 256-bit integer.
    syscall_secp256k1_add_be, "hints_syscall_secp256k1_add_be", SYSCALL_SECP256K1_ADD_BE_ID,
    SyscallSecp256k1AddParams, p1, p2, secp256k1_add, 4, 8
);
be_point_dbl_syscall!(
    /// Big-endian twin of [`super::syscall_secp256k1_dbl`]: `p <- 2 * p` on secp256k1, every
    /// coordinate a big-endian 256-bit integer.
    syscall_secp256k1_dbl_be, "hints_syscall_secp256k1_dbl_be", SYSCALL_SECP256K1_DBL_BE_ID,
    SyscallPoint256, secp256k1_dbl, 4, 8
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_secp256r1_add`]: `p1 <- p1 + p2` on secp256r1, every
    /// coordinate a big-endian 256-bit integer.
    syscall_secp256r1_add_be, "hints_syscall_secp256r1_add_be", SYSCALL_SECP256R1_ADD_BE_ID,
    SyscallSecp256r1AddParams, p1, p2, secp256r1_add, 4, 8
);
be_point_dbl_syscall!(
    /// Big-endian twin of [`super::syscall_secp256r1_dbl`]: `p <- 2 * p` on secp256r1, every
    /// coordinate a big-endian 256-bit integer.
    syscall_secp256r1_dbl_be, "hints_syscall_secp256r1_dbl_be", SYSCALL_SECP256R1_DBL_BE_ID,
    SyscallPoint256, secp256r1_dbl, 4, 8
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_bn254_curve_add`]: `p1 <- p1 + p2` on BN254, every
    /// coordinate a big-endian 256-bit integer.
    syscall_bn254_curve_add_be, "hints_syscall_bn254_curve_add_be", SYSCALL_BN254_CURVE_ADD_BE_ID,
    SyscallBn254CurveAddParams, p1, p2, bn254_curve_add, 4, 8
);
be_point_dbl_syscall!(
    /// Big-endian twin of [`super::syscall_bn254_curve_dbl`]: `p <- 2 * p` on BN254, every
    /// coordinate a big-endian 256-bit integer.
    syscall_bn254_curve_dbl_be, "hints_syscall_bn254_curve_dbl_be", SYSCALL_BN254_CURVE_DBL_BE_ID,
    SyscallPoint256, bn254_curve_dbl, 4, 8
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_bn254_complex_add`]: `f1 <- f1 + f2` in the BN254 Fp2,
    /// every component a big-endian 256-bit integer.
    syscall_bn254_complex_add_be, "hints_syscall_bn254_complex_add_be",
    SYSCALL_BN254_COMPLEX_ADD_BE_ID, SyscallBn254ComplexAddParams, f1, f2, bn254_complex_add, 4, 8
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_bn254_complex_sub`]: `f1 <- f1 - f2` in the BN254 Fp2,
    /// every component a big-endian 256-bit integer.
    syscall_bn254_complex_sub_be, "hints_syscall_bn254_complex_sub_be",
    SYSCALL_BN254_COMPLEX_SUB_BE_ID, SyscallBn254ComplexSubParams, f1, f2, bn254_complex_sub, 4, 8
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_bn254_complex_mul`]: `f1 <- f1 * f2` in the BN254 Fp2,
    /// every component a big-endian 256-bit integer.
    syscall_bn254_complex_mul_be, "hints_syscall_bn254_complex_mul_be",
    SYSCALL_BN254_COMPLEX_MUL_BE_ID, SyscallBn254ComplexMulParams, f1, f2, bn254_complex_mul, 4, 8
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_bls12_381_curve_add`]: `p1 <- p1 + p2` on BLS12-381,
    /// every coordinate a big-endian 384-bit integer.
    syscall_bls12_381_curve_add_be, "hints_syscall_bls12_381_curve_add_be",
    SYSCALL_BLS12_381_CURVE_ADD_BE_ID, SyscallBls12_381CurveAddParams, p1, p2, bls12_381_curve_add,
    6, 12
);
be_point_dbl_syscall!(
    /// Big-endian twin of [`super::syscall_bls12_381_curve_dbl`]: `p <- 2 * p` on BLS12-381, every
    /// coordinate a big-endian 384-bit integer.
    syscall_bls12_381_curve_dbl_be, "hints_syscall_bls12_381_curve_dbl_be",
    SYSCALL_BLS12_381_CURVE_DBL_BE_ID, SyscallPoint384, bls12_381_curve_dbl, 6, 12
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_bls12_381_complex_add`]: `f1 <- f1 + f2` in the
    /// BLS12-381 Fp2, every component a big-endian 384-bit integer.
    syscall_bls12_381_complex_add_be, "hints_syscall_bls12_381_complex_add_be",
    SYSCALL_BLS12_381_COMPLEX_ADD_BE_ID, SyscallBls12_381ComplexAddParams, f1, f2,
    bls12_381_complex_add, 6, 12
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_bls12_381_complex_sub`]: `f1 <- f1 - f2` in the
    /// BLS12-381 Fp2, every component a big-endian 384-bit integer.
    syscall_bls12_381_complex_sub_be, "hints_syscall_bls12_381_complex_sub_be",
    SYSCALL_BLS12_381_COMPLEX_SUB_BE_ID, SyscallBls12_381ComplexSubParams, f1, f2,
    bls12_381_complex_sub, 6, 12
);
be_point_add_syscall!(
    /// Big-endian twin of [`super::syscall_bls12_381_complex_mul`]: `f1 <- f1 * f2` in the
    /// BLS12-381 Fp2, every component a big-endian 384-bit integer.
    syscall_bls12_381_complex_mul_be, "hints_syscall_bls12_381_complex_mul_be",
    SYSCALL_BLS12_381_COMPLEX_MUL_BE_ID, SyscallBls12_381ComplexMulParams, f1, f2,
    bls12_381_complex_mul, 6, 12
);
