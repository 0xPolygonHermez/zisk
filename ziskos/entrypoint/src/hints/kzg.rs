use crate::hints::{HINT_QUEUE, hint::Hint, macros::{concat_hint_bytes, register_hint_meta}};

const KZG_VERIFY_PROOF_HINT_ID: u32 = 0x0600;

#[no_mangle]
pub unsafe extern "C" fn hint_verify_kzg_proof(z: *const u8, y: *const u8, commitment: *const u8, proof: *const u8) {
    let z_bytes: &[u8; 32] = &*(z as *const [u8; 32]);
    let y_bytes: &[u8; 32] = &*(y as *const [u8; 32]);
    let commitment_bytes: &[u8; 48] = &*(commitment as *const [u8; 48]);
    let proof_bytes: &[u8; 48] = &*(proof as *const [u8; 48]);

    let slice_bytes = concat_hint_bytes!(0; 32 + 32 + 48 + 48; z_bytes, y_bytes, commitment_bytes, proof_bytes);

    HINT_QUEUE.push(
        Hint::new(KZG_VERIFY_PROOF_HINT_ID, &slice_bytes, slice_bytes.len(), true)
    );
}

register_hint_meta!(verify_kzg_proof, KZG_VERIFY_PROOF_HINT_ID);
