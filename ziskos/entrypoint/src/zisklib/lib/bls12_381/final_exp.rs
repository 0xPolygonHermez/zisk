//! Final exponentiation for BLS12-381

use crate::zisklib::is_one;

use super::{
    cyclotomic::{
        exp_by_x_cyclo_bls12_381, exp_by_xdiv3_cyclo_bls12_381, exp_by_xone_cyclo_bls12_381,
    },
    fp12::{
        conjugate_fp12_bls12_381, exp_fp12_bls12_381, frobenius1_fp12_bls12_381,
        frobenius2_fp12_bls12_381, inv_fp12_bls12_381, mul_fp12_bls12_381,
    },
};

/// |x| for the BLS12-381 seed x = -0xd201000000010000
const X_ABS: u64 = 0xd201000000010000;

/// Checks a pairing product is 1 through the residue witness (Novakovic–Eagen, eprint 2024/640)
/// carried through the Miller loop: `f` = g·(c⁻¹)^|x|, with g the Miller loop before its final
/// conjugation, so g·s = c^λ for λ = p - x (a multiple of r) becomes
/// f·s·(c⁻¹)ᵖ = 1 (which also rules out c⁻¹ = 0). A pairing product is 1 exactly when
/// g^((p¹²-1)/r) = 1: conjugation is the p⁶-th power, which keeps or inverts that value. When the
/// witness fails, g = f·c^|x| takes the full final exponentiation.
pub(crate) fn miller_loop_witness_is_one_bls12_381(
    f: &[u64; 72],
    c_inv: &[u64; 72],
    s: &[u64; 36],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let mut s12 = [0u64; 72];
    s12[..36].copy_from_slice(s);
    let fs = mul_fp12_bls12_381(
        f,
        &s12,
        #[cfg(feature = "hints")]
        hints,
    );
    let c_inv_p = frobenius1_fp12_bls12_381(
        c_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    if is_one(&mul_fp12_bls12_381(
        &fs,
        &c_inv_p,
        #[cfg(feature = "hints")]
        hints,
    )) {
        return true;
    }

    let c = inv_fp12_bls12_381(
        c_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    let c_x = exp_fp12_bls12_381(
        X_ABS,
        &c,
        #[cfg(feature = "hints")]
        hints,
    );
    let g = mul_fp12_bls12_381(
        f,
        &c_x,
        #[cfg(feature = "hints")]
        hints,
    );
    is_one(&final_exp_bls12_381(
        &g,
        #[cfg(feature = "hints")]
        hints,
    ))
}

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

// ==================== C FFI Functions ====================

/// Final exponentiation for BLS12-381: raises an Fp12 element to the power (p¹²-1)/r.
///
/// # Safety
/// - `f_ptr` must point to a valid `[u64; 72]` array (Fp12 element, little-endian limbs)
/// - `result_ptr` must point to a writable `[u64; 72]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_final_exp_bls12_381_c")]
pub unsafe extern "C" fn final_exp_bls12_381_c(
    f_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let f = &*(f_ptr as *const [u64; 72]);
    let result = &mut *(result_ptr as *mut [u64; 72]);
    *result = final_exp_bls12_381(
        f,
        #[cfg(feature = "hints")]
        hints,
    );
}
