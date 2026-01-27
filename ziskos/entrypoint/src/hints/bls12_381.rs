use crate::hints::macros::{define_hint, define_hint_pairs};

const BLS12_381_G1_ADD_HINT_ID: u32 = 0x0400;
const BLS12_381_G1_MSM_HINT_ID: u32 = 0x0401;
const BLS12_381_G2_ADD_HINT_ID: u32 = 0x0405;
const BLS12_381_G2_MSM_HINT_ID: u32 = 0x0406;
const BLS12_381_PAIRING_CHECK_HINT_ID: u32 = 0x040A;
const BLS12_381_FP_TO_G1_HINT_ID: u32 = 0x0410;
const BLS12_381_FP2_TO_G2_HINT_ID: u32 = 0x0411;

define_hint! {
    bls12_381_g1_add => {
        hint_id: BLS12_381_G1_ADD_HINT_ID,
        params: (a: 96, b: 96),
        is_result: false,
    }
}

define_hint_pairs! {
    bls12_381_g1_msm => {
        hint_id: BLS12_381_G1_MSM_HINT_ID,
        pair_len: 96 + 32,
        is_result: false,
    }
}

define_hint! {
    bls12_381_g2_add => {
        hint_id: BLS12_381_G2_ADD_HINT_ID,
        params: (a: 192, b: 192),
        is_result: false,
    }
}

define_hint_pairs! {
    bls12_381_g2_msm => {
        hint_id: BLS12_381_G2_MSM_HINT_ID,
        pair_len: 192 + 32,
        is_result: false,
    }
}

define_hint_pairs! {
    bls12_381_pairing_check => {
        hint_id: BLS12_381_PAIRING_CHECK_HINT_ID,
        pair_len: 96 + 192,
        is_result: false,
    }
}

define_hint! {
    bls12_381_fp_to_g1 => {
        hint_id: BLS12_381_FP_TO_G1_HINT_ID,
        params: (fp: 48),
        is_result: false,
    }
}

define_hint! {
    bls12_381_fp2_to_g2 => {
        hint_id: BLS12_381_FP2_TO_G2_HINT_ID,
        params: (fp2: 96),
        is_result: false,
    }
}
