//! Final exponentiation for the pairings over BN254

use super::{
    cyclotomic::exp_by_x_cyclo_bn254,
    fp12::{
        conjugate_fp12_bn254, frobenius1_fp12_bn254, frobenius2_fp12_bn254, frobenius3_fp12_bn254,
        inv_fp12_bn254, mul_fp12_bn254, square_fp12_bn254,
    },
};

// TODO: The final exp could be optimized by using the optimizations described in https://eprint.iacr.org/2024/640.pdf
// However, I dont think its a good idea in general to optimize verification "at all costs".

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
