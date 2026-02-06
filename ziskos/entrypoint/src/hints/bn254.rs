use crate::hints::macros::{define_hint, define_hint_pairs};
use zisk_common::{HINT_BN254_G1_ADD, HINT_BN254_G1_MUL, HINT_BN254_PAIRING_CHECK};

define_hint! {
    bn254_g1_add => {
        hint_id: HINT_BN254_G1_ADD,
        params: (p1: 64, p2: 64),
        is_result: false,
    }
}

define_hint! {
    bn254_g1_mul => {
        hint_id: HINT_BN254_G1_MUL,
        params: (point: 64, scalar: 32),
        is_result: false,
    }
}

define_hint_pairs! {
    bn254_pairing_check => {
        hint_id: HINT_BN254_PAIRING_CHECK,
        pair_len: 64 + 128,
        is_result: false,
    }
}
