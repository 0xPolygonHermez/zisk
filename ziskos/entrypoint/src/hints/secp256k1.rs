use crate::hints::macros::define_hint;
use zisk_common::{HINT_SECP256K1_ECDSA_VERIFY, HINT_SECP256K1_ECRECOVER};

define_hint! {
    secp256k1_ecrecover => {
        hint_id: HINT_SECP256K1_ECRECOVER,
        params: (sig: 64, recid: 8, msg: 32),
        is_result: false,
    }
}

define_hint! {
    secp256k1_ecdsa_verify => {
        hint_id: HINT_SECP256K1_ECDSA_VERIFY,
        params: (sig: 64, msg: 32, pk: 64),
        is_result: false,
    }
}
