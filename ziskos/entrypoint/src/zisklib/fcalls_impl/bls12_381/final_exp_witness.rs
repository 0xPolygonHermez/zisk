//! Residue witness for the BLS12-381 pairing check (Novakovic–Eagen, eprint 2024/640), computed
//! with blst on the guest's own Miller loop value.
//!
//! For a Miller loop output f with f^((p¹²-1)/r) = 1 it finds c ∈ Fp12 and s ∈ Fp6 with
//! f·s = c^λ, λ = p - x. (p¹²-1)/r = 27·m·h with m = (1-x)/3, all coprime: s cancels f's
//! components of order dividing 27 and m, and c is the λ-th root of the rest, of order dividing
//! h, where λ is invertible. Both are powers of f.

use std::sync::OnceLock;

use blst::{
    blst_fp12, blst_fp12_frobenius_map, blst_fp12_inverse, blst_fp12_is_equal, blst_fp12_mul,
    blst_fp12_one, blst_fp12_sqr, blst_fp_from_uint64, blst_uint64_from_fp,
};
use num_bigint::BigUint;

use super::super::utils::{frobenius_table, pow_frob, Fp12Ops};
use super::constants::P;
use crate::zisklib::{
    miller_loop_batch_with_witness_bls12_381, BLS12_381_FINAL_EXP_WITNESS_MAX_PAIRS,
};

/// BLS12-381 seed x = -0xd201000000010000.
const X_ABS: u64 = 0xd201000000010000;

/// (S, C) with s = f^S and c = f^C.
fn exponents() -> &'static (BigUint, BigUint) {
    static EXPONENTS: OnceLock<(BigUint, BigUint)> = OnceLock::new();
    EXPONENTS.get_or_init(|| {
        let p: &BigUint = &P;
        let x_abs = BigUint::from(X_ABS);
        let r = x_abs.pow(4u32) - x_abs.pow(2u32) + 1u32;
        let n = p.pow(12u32) - 1u32;
        let m = (&x_abs + 1u32) / 3u32;
        let h = &n / &r / (27u32 * &m);
        let lambda = p + &x_abs;
        let inv = |a: &BigUint, n: &BigUint| a.modinv(n).expect("coprime");
        // f^h has order 27·m; raising it to 27·u (u = -(27h)⁻¹ mod m) and m·v (v = -(mh)⁻¹ mod 27)
        // gives the inverses of f's m and 27 components
        let u = &m - inv(&(27u32 * &h), &m);
        let v = 27u32 - inv(&(&m * &h), &BigUint::from(27u32));
        let s = &h * (27u32 * u + &m * v);
        let c = ((1u32 + &s) * inv(&lambda, &h)) % &n;
        (s, c)
    })
}

impl Fp12Ops for blst_fp12 {
    fn one() -> Self {
        unsafe { *blst_fp12_one() }
    }
    fn mul(&self, other: &Self) -> Self {
        let mut out = Self::default();
        unsafe { blst_fp12_mul(&mut out, self, other) };
        out
    }
    fn sqr(&self) -> Self {
        let mut out = Self::default();
        unsafe { blst_fp12_sqr(&mut out, self) };
        out
    }
    fn frobenius(&self) -> Self {
        let mut out = Self::default();
        unsafe { blst_fp12_frobenius_map(&mut out, self, 1) };
        out
    }
}

fn to_blst(a: &[u64; 72]) -> blst_fp12 {
    let mut out = blst_fp12::default();
    for (n, limbs) in a.chunks(6).enumerate() {
        let fp = &mut out.fp6[n / 6].fp2[n / 2 % 3].fp[n % 2];
        unsafe { blst_fp_from_uint64(fp, limbs.as_ptr()) };
    }
    out
}

fn from_blst(a: &blst_fp12) -> [u64; 72] {
    let mut out = [0u64; 72];
    for (n, limbs) in out.chunks_mut(6).enumerate() {
        unsafe { blst_uint64_from_fp(limbs.as_mut_ptr(), &a.fp6[n / 6].fp2[n / 2 % 3].fp[n % 2]) };
    }
    out
}

/// (c⁻¹, s) for the Miller loop of the pairs before its final conjugation, or zeros when their
/// pairing product is not 1 (the witness relation f·s = cᵖ·c^|x| fails).
pub fn bls12_381_final_exp_witness(
    g1_points: &[[u64; 12]],
    g2_points: &[[u64; 24]],
) -> ([u64; 72], [u64; 36]) {
    let f = to_blst(&miller_loop_batch_with_witness_bls12_381(
        g1_points,
        g2_points,
        None,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    ));
    let (s_exp, c_exp) = exponents();
    let tab = frobenius_table(&f);
    let s = pow_frob(&tab, &P, s_exp);
    let c = pow_frob(&tab, &P, c_exp);
    let mut c_x = c;
    for bit in (0..63).rev() {
        c_x = c_x.sqr();
        if (X_ABS >> bit) & 1 == 1 {
            c_x = c_x.mul(&c);
        }
    }
    if !unsafe { blst_fp12_is_equal(&c.frobenius().mul(&c_x), &f.mul(&s)) } {
        return ([0; 72], [0; 36]);
    }
    let mut c_inv = blst_fp12::default();
    unsafe { blst_fp12_inverse(&mut c_inv, &c) };
    (from_blst(&c_inv), from_blst(&s)[0..36].try_into().unwrap())
}

pub fn fcall_bls12_381_final_exp_witness(params: &[u64], results: &mut [u64]) -> i64 {
    let n = params[0] as usize;
    if n > BLS12_381_FINAL_EXP_WITNESS_MAX_PAIRS {
        return -1;
    }
    let (g1, g2): (Vec<[u64; 12]>, Vec<[u64; 24]>) = params[1..1 + 36 * n]
        .chunks(36)
        .map(|pair| -> ([u64; 12], [u64; 24]) {
            (pair[..12].try_into().unwrap(), pair[12..].try_into().unwrap())
        })
        .unzip();
    let (c_inv, s) = bls12_381_final_exp_witness(&g1, &g2);
    results[0..72].copy_from_slice(&c_inv);
    results[72..108].copy_from_slice(&s);
    108
}
