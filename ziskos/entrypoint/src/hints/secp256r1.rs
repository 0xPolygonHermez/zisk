use crate::hints::macros::define_hint;

const SECP256R1_ECDSA_VERIFY_HINT_ID: u32 = 0x0301;

define_hint! {
    secp256r1_ecdsa_verify => {
        hint_id: SECP256R1_ECDSA_VERIFY_HINT_ID,
        params: (msg: 32, sig: 64, pk: 64),
        is_result: false,
    }
}
