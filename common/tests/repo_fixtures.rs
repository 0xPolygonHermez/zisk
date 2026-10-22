//! Tests over real proofs committed elsewhere in the repository. Cargo does not package
//! sibling crates' files, so this target is excluded from the published crate.

use zisk_common::{CommonError, Proof, ProofBody};

/// With no key passed, a leaf verifies under the release key, never the one it carries:
/// a real proof whose claimed key is garbage still verifies, and a tampered one fails.
#[test]
fn verify_defaults_to_the_release_key_not_the_proofs_own() {
    let bytes = include_bytes!("../../verifier/tests/fixtures/poseidon2_compressed.bin");
    let mut proof = Proof::from_bytes(bytes).unwrap();
    if let ProofBody::Vadcop { zisk_vk, .. } = &mut proof.body {
        *zisk_vk = vec![1, 2, 3, 4];
    }
    proof.verify().unwrap();

    if let ProofBody::Vadcop { proof: words, .. } = &mut proof.body {
        words[7] ^= 1;
    }
    assert!(matches!(proof.verify().unwrap_err(), CommonError::NotVerified));
}
