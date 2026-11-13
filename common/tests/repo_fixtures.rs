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

/// A real proof whose `program_vk` was swapped in transit (bincode, the coordinator ↔ client
/// encoding) must not verify while naming a program its publics do not commit to.
#[test]
fn verify_rejects_a_program_vk_swapped_in_bincode() {
    let bytes = include_bytes!("../../verifier/tests/fixtures/poseidon2_leaf.bin");
    let mut proof = Proof::from_bytes(bytes).unwrap();
    proof.verify().unwrap();

    proof.program_vk.vk[0] ^= 1;
    let proof = Proof::from_bincode(&proof.to_bincode().unwrap()).unwrap();
    let err = proof.verify().unwrap_err();
    assert!(
        matches!(&err, CommonError::InvalidProof(m) if m.contains("program_vk differs")),
        "got: {err:?}"
    );
}
