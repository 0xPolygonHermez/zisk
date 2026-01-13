use crate::hints::{
    macros::define_hint,
    types::{
        HINT_ADD_BLS12_381, HINT_ADD_TWIST_BLS12_381, HINT_DECOMPRESS_BLS12_381, HINT_DECOMPRESS_TWIST_BLS12_381, HINT_FINAL_EXP_BLS12_381, HINT_IS_ON_CURVE_BLS12_381, HINT_IS_ON_CURVE_TWIST_BLS12_381, HINT_IS_ON_SUBGROUP_BLS12_381, HINT_IS_ON_SUBGROUP_TWIST_BLS12_381, HINT_MILLER_LOOP_BLS12_381, HINT_MUL_FP12_BLS12_381, HINT_SCALAR_MUL_BLS12_381, HINT_SCALAR_MUL_TWIST_BLS12_381
    }
};

// === mul_fp12_bls12_381 (a, b) ===

define_hint! {
    variant MulFp12Bls12_381 { a: u64;72, b: u64;72 }
    hint(
        fn hint_mul_fp12_bls12_381,
        ty = HINT_MUL_FP12_BLS12_381
    );
}

// === is_on_curve_bls12_381 (p) ===

define_hint! {
    variant IsOnCurveBls12_381 { p: u64;12 }
    hint(
        fn hint_is_on_curve_bls12_381,
        ty = HINT_IS_ON_CURVE_BLS12_381
    );
}

// === is_on_subgroup_bls12_381 (p) ===

define_hint! {
    variant IsOnSubgroupBls12_381 { p: u64;12 }
    hint(
        fn hint_is_on_subgroup_bls12_381,
        ty = HINT_IS_ON_SUBGROUP_BLS12_381
    );
}

// === add_bls12_381_c (p1, p2) ===

define_hint! {
    variant AddBls12_381 { p1: u64;12, p2: u64;12 }
    hint(
        fn hint_add_bls12_381,
        ty = HINT_ADD_BLS12_381
    );
}

// === scalar_mul_bls12_381 (p, k) ===

define_hint! {
    variant ScalarMulBls12_381 { p: u64;12, k: u64;6 }
    hint(
        fn hint_scalar_mul_bls12_381,
        ty = HINT_SCALAR_MUL_BLS12_381
    );
}

// === is_on_curve_twist_bls12_381 (p) ===

define_hint! {
    variant IsOnCurveTwistBls12_381 { p: u64;24 }
    hint(
        fn hint_is_on_curve_twist_bls12_381,
        ty = HINT_IS_ON_CURVE_TWIST_BLS12_381
    );
}

// === is_on_subgroup_twist_bls12_381 (p) ===

define_hint! {
    variant IsOnSubgroupTwistBls12_381 { p: u64;24 }
    hint(
        fn hint_is_on_subgroup_twist_bls12_381,
        ty = HINT_IS_ON_SUBGROUP_TWIST_BLS12_381
    );
}

// === add_twist_bls12_381 (p1, p2) ===

define_hint! {
    variant AddTwistBls12_381 { p1: u64;24, p2: u64;24 }
    hint(
        fn hint_add_twist_bls12_381,
        ty = HINT_ADD_TWIST_BLS12_381
    );
}

// === scalar_mul_twist_bls12_381 (p, k) ===

define_hint! {
    variant ScalarMulTwistBls12_381 { p: u64;24, k: u64;6 }
    hint(
        fn hint_scalar_mul_twist_bls12_381,
        ty = HINT_SCALAR_MUL_TWIST_BLS12_381
    );
}

// === miller_loop_bls12_381 (q, p) ===

define_hint! {
    variant MillerLoopBls12_381 { p: u64;12, q: u64;24 }
    hint(
        fn hint_miller_loop_bls12_381,
        ty = HINT_MILLER_LOOP_BLS12_381
    );
}

// === final_exp_bls12_381 (f) ===

define_hint! {
    variant FinalExpBls12_381 { f: u64;72 }
    hint(
        fn hint_final_exp_bls12_381,
        ty = HINT_FINAL_EXP_BLS12_381
    );
}

// === decompress_bls12_381 (input) ===

define_hint! {
    variant DecompressBls12_381 { input: u8;48 }
    hint(
        fn hint_decompress_bls12_381,
        ty = HINT_DECOMPRESS_BLS12_381
    );
}

// === decompress_twist_bls12_381 (input) ===

define_hint! {
    variant DecompressTwistBls12_381 { input: u8;96 }
    hint(
        fn hint_decompress_twist_bls12_381,
        ty = HINT_DECOMPRESS_TWIST_BLS12_381
    );
}