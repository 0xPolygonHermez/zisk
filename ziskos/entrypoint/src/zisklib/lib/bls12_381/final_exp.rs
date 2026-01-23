//! Final exponentiation for BLS12-381

use super::{
    cyclotomic::{
        exp_by_x_cyclo_bls12_381, exp_by_xdiv3_cyclo_bls12_381, exp_by_xone_cyclo_bls12_381,
    },
    fp12::{
        conjugate_fp12_bls12_381, frobenius1_fp12_bls12_381, frobenius2_fp12_bls12_381,
        inv_fp12_bls12_381, mul_fp12_bls12_381,
    },
};

// TODO: The final exp could be optimized by using the optimizations described in https://eprint.iacr.org/2024/640.pdf
// However, I dont think its a good idea in general to optimize verification "at all costs".

/// Given f ∈ Fp12*, computes f^((p¹²-1)/r) ∈ Fp12*
///
/// Note: Unoptimized for the case f == 1
pub fn final_exp_bls12_381(
    f: &[u64; 72],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 72] {
    //////////////////
    // The easy part: exp by (p^6-1)(p^2+1)
    //////////////////

    // f^(p^6-1) = f̅·f⁻¹
    let f_conj = conjugate_fp12_bls12_381(
        f,
        #[cfg(feature = "hints")]
        hints,
    );
    let f_inv = inv_fp12_bls12_381(
        f,
        #[cfg(feature = "hints")]
        hints,
    );
    let easy1 = mul_fp12_bls12_381(
        &f_conj,
        &f_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // easy1^(p²-1) = easy1^p²·easy1
    let mut m = frobenius2_fp12_bls12_381(
        &easy1,
        #[cfg(feature = "hints")]
        hints,
    );
    m = mul_fp12_bls12_381(
        &m,
        &easy1,
        #[cfg(feature = "hints")]
        hints,
    );

    //////////////////
    // The hard part: exp by (p⁴-p²+1)/r
    //////////////////

    // f = m^{(x+1)/3}
    let mut f = exp_by_xdiv3_cyclo_bls12_381(
        &m,
        #[cfg(feature = "hints")]
        hints,
    );

    // f = f^(x+1)
    f = exp_by_xone_cyclo_bls12_381(
        &f,
        #[cfg(feature = "hints")]
        hints,
    );

    // f1 = f^p, f2 = f̅^x
    let f1 = frobenius1_fp12_bls12_381(
        &f,
        #[cfg(feature = "hints")]
        hints,
    );
    let f2 = exp_by_x_cyclo_bls12_381(
        &conjugate_fp12_bls12_381(
            &f,
            #[cfg(feature = "hints")]
            hints,
        ),
        #[cfg(feature = "hints")]
        hints,
    );

    // f = f1*f2
    let f = mul_fp12_bls12_381(
        &f1,
        &f2,
        #[cfg(feature = "hints")]
        hints,
    );

    // f1 = (f^x)^x, f2 = f^p², f3 = f̅
    let f1 = exp_by_x_cyclo_bls12_381(
        &exp_by_x_cyclo_bls12_381(
            &f,
            #[cfg(feature = "hints")]
            hints,
        ),
        #[cfg(feature = "hints")]
        hints,
    );
    let f2 = frobenius2_fp12_bls12_381(
        &f,
        #[cfg(feature = "hints")]
        hints,
    );
    let f3 = conjugate_fp12_bls12_381(
        &f,
        #[cfg(feature = "hints")]
        hints,
    );

    // f = f1*f2*f3*m
    let mut f = mul_fp12_bls12_381(
        &f1,
        &f2,
        #[cfg(feature = "hints")]
        hints,
    );
    f = mul_fp12_bls12_381(
        &f,
        &f3,
        #[cfg(feature = "hints")]
        hints,
    );
    f = mul_fp12_bls12_381(
        &f,
        &m,
        #[cfg(feature = "hints")]
        hints,
    );

    f
}
