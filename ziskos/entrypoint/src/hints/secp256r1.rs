use crate::hints::macros::define_hint;
use zisk_common::HINT_SECP256R1_ECDSA_VERIFY;

define_hint! {
    secp256r1_ecdsa_verify => {
        hint_id: HINT_SECP256R1_ECDSA_VERIFY,
        params: (msg: 32, sig: 64, pk: 64),
        is_result: false,
    }
}
