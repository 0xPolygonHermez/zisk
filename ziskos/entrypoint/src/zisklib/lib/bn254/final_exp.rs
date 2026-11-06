//! Final exponentiation for the pairings over BN254

use crate::zisklib::{eq, is_one};

use super::{
    cyclotomic::exp_by_x_cyclo_bn254,
    fp12::{
        conjugate_fp12_bn254, exp_fp12_bn254, frobenius1_fp12_bn254, frobenius2_fp12_bn254,
        frobenius3_fp12_bn254, inv_fp12_bn254, mul_fp12_bn254, square_fp12_bn254,
    },
};

/// 3x+1 for the BN254 seed x = 4965661367192848881, so that 6x+2 = 2·(3x+1) fits a u64 exponent
const THREE_X_PLUS_ONE: u64 = 14896984101578546644;

/// Checks a pairing product is 1 through the residue witness (Novakovic–Eagen, eprint 2024/640)
/// carried through the Miller loop: `f` = g·(c⁻¹)^(6x+2), with g the Miller loop and c = (c⁻¹)⁻¹
/// computed by the caller, so g·s = c^λ for λ = 6x+2+p-p²+p³ (a multiple of r) becomes
/// f·s·(c⁻¹)ᵖ·(c⁻¹)^(p³) = (c⁻¹)^(p²). When the witness fails, g = f·c^(6x+2) takes the full
/// final exponentiation.
pub(crate) fn miller_loop_witness_is_one_bn254(
    f: &[u64; 48],
    c_inv: &[u64; 48],
    c: &[u64; 48],
    s: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let mut s12 = [0u64; 48];
    s12[..24].copy_from_slice(s);
    let fs = mul_fp12_bn254(
        f,
        &s12,
        #[cfg(feature = "hints")]
        hints,
    );
    let c_inv_p2 = frobenius2_fp12_bn254(
        c_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    // (c⁻¹)ᵖ·(c⁻¹)^(p³) = (c⁻¹·(c⁻¹)^(p²))ᵖ
    let c_inv_p_p3 = frobenius1_fp12_bn254(
        &mul_fp12_bn254(
            c_inv,
            &c_inv_p2,
            #[cfg(feature = "hints")]
            hints,
        ),
        #[cfg(feature = "hints")]
        hints,
    );
    if eq(
        &mul_fp12_bn254(
            &fs,
            &c_inv_p_p3,
            #[cfg(feature = "hints")]
            hints,
        ),
        &c_inv_p2,
    ) {
        return true;
    }

    let c_6x2 = square_fp12_bn254(
        &exp_fp12_bn254(
            THREE_X_PLUS_ONE,
            c,
            #[cfg(feature = "hints")]
            hints,
        ),
        #[cfg(feature = "hints")]
        hints,
    );
    let g = mul_fp12_bn254(
        f,
        &c_6x2,
        #[cfg(feature = "hints")]
        hints,
    );
    is_one(&final_exp_bn254(
        &g,
        #[cfg(feature = "hints")]
        hints,
    ))
}

/// Given f ∈ Fp12*, computes f^((p¹²-1)/r) ∈ Fp12*
pub fn final_exp_bn254(f: &[u64; 48], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 48] {
    //////////////////
    // The easy part: exp by (p^6-1)(p^2+1)
    //////////////////

    // f^(p^6-1) = f̅·f⁻¹
    let f_conj = conjugate_fp12_bn254(
        f,
        #[cfg(feature = "hints")]
        hints,
    );
    let f_inv = inv_fp12_bn254(
        f,
        #[cfg(feature = "hints")]
        hints,
    );
    let easy1 = mul_fp12_bn254(
        &f_conj,
        &f_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // easy1^(p²-1) = easy1^p²·easy1
    let mut m = frobenius2_fp12_bn254(
        &easy1,
        #[cfg(feature = "hints")]
        hints,
    );
    m = mul_fp12_bn254(
        &m,
        &easy1,
        #[cfg(feature = "hints")]
        hints,
    );

    //////////////////
    // The hard part: exp by (p⁴-p²+1)/r
    //////////////////

    // m^x, (m^x)^x, (m^{x²})^x
    let mx = exp_by_x_cyclo_bn254(
        &m,
        #[cfg(feature = "hints")]
        hints,
    );
    let mxx = exp_by_x_cyclo_bn254(
        &mx,
        #[cfg(feature = "hints")]
        hints,
    );
    let mxxx = exp_by_x_cyclo_bn254(
        &mxx,
        #[cfg(feature = "hints")]
        hints,
    );

    // m^p, m^p², m^p³, (m^x)^p, (m^x²)^p, (m^x³)^p, (m^x²)^p²
    let mp = frobenius1_fp12_bn254(
        &m,
        #[cfg(feature = "hints")]
        hints,
    );
    let mpp = frobenius2_fp12_bn254(
        &m,
        #[cfg(feature = "hints")]
        hints,
    );
    let mppp = frobenius3_fp12_bn254(
        &m,
        #[cfg(feature = "hints")]
        hints,
    );
    let mxp = frobenius1_fp12_bn254(
        &mx,
        #[cfg(feature = "hints")]
        hints,
    );
    let mxxp = frobenius1_fp12_bn254(
        &mxx,
        #[cfg(feature = "hints")]
        hints,
    );
    let mxxxp = frobenius1_fp12_bn254(
        &mxxx,
        #[cfg(feature = "hints")]
        hints,
    );
    let mxxpp = frobenius2_fp12_bn254(
        &mxx,
        #[cfg(feature = "hints")]
        hints,
    );

    // y1 = m^p·m^p²·m^p³
    let mut y1 = mul_fp12_bn254(
        &mp,
        &mpp,
        #[cfg(feature = "hints")]
        hints,
    );
    y1 = mul_fp12_bn254(
        &y1,
        &mppp,
        #[cfg(feature = "hints")]
        hints,
    );

    // y2 = m̅
    let y2 = conjugate_fp12_bn254(
        &m,
        #[cfg(feature = "hints")]
        hints,
    );

    // y3 = (m^x²)^p² (already done)

    // y4 = \bar{(m^x)^p}
    let y4 = conjugate_fp12_bn254(
        &mxp,
        #[cfg(feature = "hints")]
        hints,
    );

    // y5 = \bar{m^x·(m^x²)^p}
    let mut y5 = mul_fp12_bn254(
        &mx,
        &mxxp,
        #[cfg(feature = "hints")]
        hints,
    );
    y5 = conjugate_fp12_bn254(
        &y5,
        #[cfg(feature = "hints")]
        hints,
    );
    // y6 = \bar{m^x²}
    let y6 = conjugate_fp12_bn254(
        &mxx,
        #[cfg(feature = "hints")]
        hints,
    );

    // y7 = \bar{m^x³·(m^x³)^p}
    let mut y7 = mul_fp12_bn254(
        &mxxx,
        &mxxxp,
        #[cfg(feature = "hints")]
        hints,
    );
    y7 = conjugate_fp12_bn254(
        &y7,
        #[cfg(feature = "hints")]
        hints,
    );
    // Compute y1·y2²·y3⁶·y4¹²·y5¹⁸·y6³⁰·y7³⁶ as follows
    // T11 = y7²·y5·y6
    let mut t11 = square_fp12_bn254(
        &y7,
        #[cfg(feature = "hints")]
        hints,
    );
    t11 = mul_fp12_bn254(
        &t11,
        &y5,
        #[cfg(feature = "hints")]
        hints,
    );
    t11 = mul_fp12_bn254(
        &t11,
        &y6,
        #[cfg(feature = "hints")]
        hints,
    );

    // T21 = T11·y4·y6
    let mut t21 = mul_fp12_bn254(
        &t11,
        &y4,
        #[cfg(feature = "hints")]
        hints,
    );
    t21 = mul_fp12_bn254(
        &t21,
        &y6,
        #[cfg(feature = "hints")]
        hints,
    );

    // T12 = T11·y3
    let t12 = mul_fp12_bn254(
        &t11,
        &mxxpp,
        #[cfg(feature = "hints")]
        hints,
    );
    // T22 = T21²·T12
    let mut t22 = square_fp12_bn254(
        &t21,
        #[cfg(feature = "hints")]
        hints,
    );
    t22 = mul_fp12_bn254(
        &t22,
        &t12,
        #[cfg(feature = "hints")]
        hints,
    );

    // T23 = T22²
    let t23 = square_fp12_bn254(
        &t22,
        #[cfg(feature = "hints")]
        hints,
    );

    // T24 = T23·y1
    let t24 = mul_fp12_bn254(
        &t23,
        &y1,
        #[cfg(feature = "hints")]
        hints,
    );

    // T13 = T23·y2
    let t13 = mul_fp12_bn254(
        &t23,
        &y2,
        #[cfg(feature = "hints")]
        hints,
    );

    // T14 = T13²·T24
    let mut t14 = square_fp12_bn254(
        &t13,
        #[cfg(feature = "hints")]
        hints,
    );
    t14 = mul_fp12_bn254(
        &t14,
        &t24,
        #[cfg(feature = "hints")]
        hints,
    );
    t14
}
