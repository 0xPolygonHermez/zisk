use crate::hints::macros::define_hint;
use zisk_common::{
    HINT_SECP256K1_ECDSA_ADDRESS_RECOVER, HINT_SECP256K1_ECDSA_VERIFY_ADDRESS_RECOVER,
};

define_hint! {
    secp256k1_ecdsa_address_recover => {
        hint_id: HINT_SECP256K1_ECDSA_ADDRESS_RECOVER,
        params: (sig: 64, recid: 8, msg: 32),
        is_result: false,
    }
}

define_hint! {
    secp256k1_ecdsa_verify_and_address_recover => {
        hint_id: HINT_SECP256K1_ECDSA_VERIFY_ADDRESS_RECOVER,
        params: (sig: 64, msg: 32, pk: 64),
        is_result: false,
    }
}
