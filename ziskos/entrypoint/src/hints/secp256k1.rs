use crate::hints::macros::define_hint;

const SECP256K1_ECRECOVER_HINT_ID: u32 = 0x0300;

define_hint! {
    secp256k1_ecrecover => {
        hint_id: SECP256K1_ECRECOVER_HINT_ID,
        params: (sig: 64, recid: 8, msg: 32, require_low_s: 8),
        is_result: false,
    }
}
