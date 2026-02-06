use crate::hints::macros::{define_hint, define_hint_pairs};
use zisk_common::{
    HINT_BLS12_381_FP2_TO_G2, HINT_BLS12_381_FP_TO_G1, HINT_BLS12_381_G1_ADD,
    HINT_BLS12_381_G1_MSM, HINT_BLS12_381_G2_ADD, HINT_BLS12_381_G2_MSM,
    HINT_BLS12_381_PAIRING_CHECK,
};

define_hint! {
    bls12_381_g1_add => {
        hint_id: HINT_BLS12_381_G1_ADD,
        params: (a: 96, b: 96),
        is_result: false,
    }
}

define_hint_pairs! {
    bls12_381_g1_msm => {
        hint_id: HINT_BLS12_381_G1_MSM,
        pair_len: 96 + 32,
        is_result: false,
    }
}

define_hint! {
    bls12_381_g2_add => {
        hint_id: HINT_BLS12_381_G2_ADD,
        params: (a: 192, b: 192),
        is_result: false,
    }
}

define_hint_pairs! {
    bls12_381_g2_msm => {
        hint_id: HINT_BLS12_381_G2_MSM,
        pair_len: 192 + 32,
        is_result: false,
    }
}

define_hint_pairs! {
    bls12_381_pairing_check => {
        hint_id: HINT_BLS12_381_PAIRING_CHECK,
        pair_len: 96 + 192,
        is_result: false,
    }
}

define_hint! {
    bls12_381_fp_to_g1 => {
        hint_id: HINT_BLS12_381_FP_TO_G1,
        params: (fp: 48),
        is_result: false,
    }
}

define_hint! {
    bls12_381_fp2_to_g2 => {
        hint_id: HINT_BLS12_381_FP2_TO_G2,
        params: (fp2: 96),
        is_result: false,
    }
}
