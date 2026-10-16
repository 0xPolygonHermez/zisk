//! Tests over real proofs committed elsewhere in the repository. Cargo does not package
//! sibling crates' files, so this target is excluded from the published crate.

use zisk_common::{CommonError, Proof, ProofBody};

/// The BN254 scalar field order, big-endian.
const BN254_R_BE: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93, 0xf0, 0x00, 0x00, 0x01,
];

/// With no key passed, a leaf verifies under the release key, never the one it carries:
/// a real proof whose claimed key is garbage still verifies, and a tampered one fails.
#[test]
#[ignore = "fixtures predate the current verifiers; re-prove them under the release setup"]
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

/// Every proof word must be below the BN254 scalar order, as `PlonkVerifier.sol` requires:
/// adding `r` gives another encoding the native parser reads back as the same proof.
#[test]
fn a_plonk_proof_word_at_or_above_r_is_refused() {
    let bytes = include_bytes!("../../zisk-contracts/test/plonk_proof.bin");
    let mut proof = Proof::from_bytes(bytes).unwrap();
    let r = num_bigint::BigUint::from_bytes_be(&BN254_R_BE);
    if let ProofBody::Plonk { proof_bytes, .. } = &mut proof.body {
        let word = num_bigint::BigUint::from_bytes_be(&proof_bytes[..32]) + &r;
        let be = word.to_bytes_be();
        proof_bytes[..32].fill(0);
        proof_bytes[32 - be.len()..32].copy_from_slice(&be);
    }
    let err = proof.verify().unwrap_err();
    assert!(err.to_string().contains("word 0"), "got: {err}");
    assert!(proof.to_bytes().is_err());
}

/// The contract's fixture verifies with no key passed, under the release PLONK key.
#[test]
fn the_contract_fixture_verifies_under_the_release_plonk_key() {
    let bytes = include_bytes!("../../zisk-contracts/test/plonk_proof.bin");
    Proof::from_bytes(bytes).unwrap().verify().unwrap();
}

/// CI exports Solidity calldata from the committed PLONK fixture.
#[test]
fn the_contract_fixture_loads_without_a_seed() {
    let bytes = include_bytes!("../../zisk-contracts/test/plonk_proof.bin");
    assert_eq!(Proof::from_bytes(bytes).unwrap().random_seed, None);
}
