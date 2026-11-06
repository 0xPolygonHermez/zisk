//! Residue witness for the BN254 pairing check (Novakovic–Eagen, eprint 2024/640, §4.3.2),
//! computed with arkworks on the guest's own Miller loop value.
//!
//! For a Miller loop output f with f^((p¹²-1)/r) = 1 it finds c ∈ Fp12 and s ∈ Fp6 with
//! f·s = c^λ, λ = 6x+2+p-p²+p³ = 3·r·m with m coprime to p¹²-1. s = ρⁱ, for a fixed 27th root of
//! unity ρ ∈ Fp6 that is not a cube, makes f·s a cube; c is then its λ-th root: an m-th root, an
//! r-th root (r ∤ (p¹²-1)/r) and a cube root.

use std::sync::OnceLock;

use ark_bn254::{Fq, Fq12, Fq2, Fq6};
use ark_ff::{BigInt as ArkBigInt, Field, One, PrimeField, Zero};
use num_bigint::BigUint;

use super::super::utils::{frobenius_table, pow_frob, Fp12Ops};
use super::constants::P;
use crate::zisklib::{miller_loop_batch_with_witness_bn254, BN254_FINAL_EXP_WITNESS_MAX_PAIRS};

/// BN254 seed x.
const X: u64 = 4965661367192848881;

struct Params {
    /// (p¹²-1)/3
    n_div_3: BigUint,
    /// (m⁻¹ mod p¹²-1)·(r⁻¹ mod (p¹²-1)/r)
    root_m_r: BigUint,
    /// (s+1)/3, where p¹²-1 = 27·s
    cube_root: BigUint,
    /// a 27th root of unity in Fp6 that is not a cube in Fp12, and ζ = ρ^((p¹²-1)/3)
    rho: Fq12,
    zeta: Fq12,
}

fn params() -> &'static Params {
    static PARAMS: OnceLock<Params> = OnceLock::new();
    PARAMS.get_or_init(|| {
        let p: &BigUint = &P;
        let x = BigUint::from(X);
        let r = 36u32 * x.pow(4u32) + 36u32 * x.pow(3u32) + 18u32 * x.pow(2u32) + 6u32 * &x + 1u32;
        let n = p.pow(12u32) - 1u32;
        let lambda = 6u32 * &x + 2u32 + p.pow(3u32) + p - p.pow(2u32);
        let m = &lambda / (3u32 * &r);
        let inv = |a: &BigUint, n: &BigUint| a.modinv(n).expect("coprime");
        let root_m_r = (inv(&m, &n) * inv(&r, &(&n / &r))) % &n;
        let n_div_3 = &n / 3u32;

        // ρ = g^((p⁶-1)/27) for g ∈ Fp6, with ρ⁹ ≠ 1 so that its order is 27. g needs a v term:
        // 27 ∤ p²-1, so no element of Fp2 qualifies.
        let fp6_27 = (p.pow(6u32) - 1u32) / 27u32;
        let rho = (1u64..64)
            .map(|k| {
                let g = Fq6::new(Fq2::new(Fq::from(k), Fq::one()), Fq2::one(), Fq2::zero());
                Fq12::new(g, Fq6::zero()).pow(fp6_27.to_u64_digits())
            })
            .find(|rho| !rho.pow([9u64]).is_one())
            .expect("a 27th root of unity in Fp6");
        let zeta = rho.pow(n_div_3.to_u64_digits());
        Params { n_div_3, root_m_r, cube_root: (&n / 27u32 + 1u32) / 3u32, rho, zeta }
    })
}

impl Fp12Ops for Fq12 {
    fn one() -> Self {
        <Fq12 as One>::one()
    }
    fn mul(&self, other: &Self) -> Self {
        *self * other
    }
    fn sqr(&self) -> Self {
        self.square()
    }
    fn frobenius(&self) -> Self {
        self.frobenius_map(1)
    }
}

fn to_ark(a: &[u64; 48]) -> Fq12 {
    let fq = |limbs: &[u64]| Fq::from_bigint(ArkBigInt(limbs.try_into().unwrap())).unwrap();
    Fq12::from_base_prime_field_elems(a.chunks(4).map(fq)).unwrap()
}

fn from_ark(a: &Fq12) -> [u64; 48] {
    let mut out = [0u64; 48];
    for (limbs, fq) in out.chunks_mut(4).zip(a.to_base_prime_field_elements()) {
        limbs.copy_from_slice(&fq.into_bigint().0);
    }
    out
}

/// (c⁻¹, s) for the Miller loop of the pairs, or zeros when their pairing product is not 1 (the
/// witness relation f·s·c^(p²) = c^(6x+2)·cᵖ·c^(p³) fails).
pub fn bn254_final_exp_witness(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
) -> ([u64; 48], [u64; 24]) {
    let pr = params();
    let f = to_ark(&miller_loop_batch_with_witness_bn254(
        g1_points,
        g2_points,
        None,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    ));

    // f·ρⁱ is a cube for exactly one i ∈ {0, 1, 2}: the i with f^((p¹²-1)/3)·ζⁱ = 1
    let mut y = pow_frob(&frobenius_table(&f), &P, &pr.n_div_3);
    let mut s = <Fq12 as One>::one();
    for _ in 0..2 {
        if y.is_one() {
            break;
        }
        y *= pr.zeta;
        s *= pr.rho;
    }

    // cube root of the m-th and r-th root of f·s, corrected by the power of ρ³ that makes c³ = root
    let root = pow_frob(&frobenius_table(&(f * s)), &P, &pr.root_m_r);
    let mut c = pow_frob(&frobenius_table(&root), &P, &pr.cube_root);
    let mut c3 = c.square() * c;
    let rho3 = pr.rho.pow([3u64]);
    for _ in 0..8 {
        if c3 == root {
            break;
        }
        c3 *= rho3;
        c *= pr.rho;
    }

    let (cp, cp2, cp3) = (c.frobenius_map(1), c.frobenius_map(2), c.frobenius_map(3));
    if c.pow([3 * X + 1]).square() * cp * cp3 != f * s * cp2 {
        return ([0; 48], [0; 24]);
    }
    let c_inv = c.inverse().unwrap_or_default();
    (from_ark(&c_inv), from_ark(&s)[0..24].try_into().unwrap())
}

pub fn fcall_bn254_final_exp_witness(params: &[u64], results: &mut [u64]) -> i64 {
    let n = params[0] as usize;
    if n > BN254_FINAL_EXP_WITNESS_MAX_PAIRS {
        return -1;
    }
    let (g1, g2): (Vec<[u64; 8]>, Vec<[u64; 16]>) = params[1..1 + 24 * n]
        .chunks(24)
        .map(|pair| -> ([u64; 8], [u64; 16]) {
            (pair[..8].try_into().unwrap(), pair[8..].try_into().unwrap())
        })
        .unzip();
    let (c_inv, s) = bn254_final_exp_witness(&g1, &g2);
    results[0..48].copy_from_slice(&c_inv);
    results[48..72].copy_from_slice(&s);
    72
}
