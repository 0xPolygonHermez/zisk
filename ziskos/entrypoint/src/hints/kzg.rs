use crate::hints::macros::define_hint;

const KZG_VERIFY_PROOF_HINT_ID: u32 = 0x0600;

define_hint! {
    verify_kzg_proof => {
        hint_id: KZG_VERIFY_PROOF_HINT_ID,
        params: (z: 32, y: 32, commitment: 48, proof: 48),
        is_result: false,
    }
}
