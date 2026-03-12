use crate::hints::macros::define_hint;
use zisk_common::HINT_VERIFY_KZG_PROOF;

define_hint! {
    verify_kzg_proof => {
        hint_id: HINT_VERIFY_KZG_PROOF,
        params: (z: 32, y: 32, commitment: 48, proof: 48),
        is_result: false,
    }
}
