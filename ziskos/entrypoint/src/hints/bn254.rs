use crate::hints::macros::{define_hint, define_hint_pairs};

const BN254_G1_ADD_HINT_ID: u32 = 0x0200;
const BN254_G1_MUL_HINT_ID: u32 = 0x0201;
const BN254_PAIRING_CHECK_HINT_ID: u32 = 0x0205;

define_hint! {
    bn254_g1_add => {
        hint_id: BN254_G1_ADD_HINT_ID,
        params: (p1: 64, p2: 64),
        is_result: false,
    }
}

define_hint! {
    bn254_g1_mul => {
        hint_id: BN254_G1_MUL_HINT_ID,
        params: (point: 64, scalar: 32),
        is_result: false,
    }
}

define_hint_pairs! {
    bn254_pairing_check => {
        hint_id: BN254_PAIRING_CHECK_HINT_ID,
        pair_len: 64 + 128,
        is_result: false,
    }
}
