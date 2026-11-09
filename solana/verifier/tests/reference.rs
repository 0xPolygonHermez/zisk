use ark_bn254::{Fq, Fr};
use ark_ff::{BigInteger, PrimeField};
use serde_json::Value;
use zisk_solana_verifier::{verify, VerifyError};

fn fixture() -> ([u8; 32], Vec<u8>, Vec<u8>) {
    let v: Value = serde_json::from_str(include_str!("../../fixtures/blake3.json")).unwrap();
    let bytes = |k: &str| hex::decode(v[k].as_str().unwrap().trim_start_matches("0x")).unwrap();
    (bytes("programVK").try_into().unwrap(), bytes("publicValues"), bytes("proofBytes"))
}

#[test]
fn real_release_proof_and_every_word_mutation() {
    let (guest, outputs, proof) = fixture();
    assert_eq!(verify(&guest, &outputs, &proof), Ok(()));
    for word in 0..211 {
        let mut changed = proof.clone();
        changed[word * 32 + 31] ^= 1;
        assert!(verify(&guest, &outputs, &changed).is_err(), "word {word}");
    }
}

#[test]
fn statement_binding_and_lengths() {
    let (guest, outputs, proof) = fixture();
    let mut other = guest;
    other[31] ^= 1;
    assert!(verify(&other, &outputs, &proof).is_err());
    for position in [0, 4, 7, 511] {
        let mut changed = outputs.clone();
        changed[position] ^= 1;
        assert!(verify(&guest, &changed, &proof).is_err());
    }
    for n in [0, 1, 575, 6751, 6753, 7000] {
        let mut changed = proof.clone();
        changed.resize(n, 0);
        assert_eq!(verify(&guest, &outputs, &changed), Err(VerifyError::InvalidLength));
    }
    for n in [0, 511, 513, 1024] {
        let mut changed = outputs.clone();
        changed.resize(n, 0);
        assert_eq!(verify(&guest, &changed, &proof), Err(VerifyError::InvalidLength));
    }
}

#[test]
fn noncanonical_scalars_and_points() {
    let (guest, outputs, proof) = fixture();
    for word in [18, 50, 208, 209, 210] {
        let mut changed = proof.clone();
        changed[word * 32..(word + 1) * 32].copy_from_slice(&Fr::MODULUS.to_bytes_be());
        assert_eq!(verify(&guest, &outputs, &changed), Err(VerifyError::InvalidEncoding));
    }
    for point in 0..9 {
        for bad in [vec![0; 64], [Fq::MODULUS.to_bytes_be(), vec![0; 32]].concat(), vec![255; 64]] {
            let mut changed = proof.clone();
            changed[point * 64..(point + 1) * 64].copy_from_slice(&bad);
            assert_eq!(verify(&guest, &outputs, &changed), Err(VerifyError::InvalidEncoding));
        }
    }
    // Canonical Goldilocks limbs are part of the release statement encoding.
    let mut changed = outputs.clone();
    changed[..8].copy_from_slice(&0xffff_ffff_0000_0001u64.to_le_bytes());
    assert_eq!(verify(&guest, &changed, &proof), Err(VerifyError::InvalidEncoding));
}

#[cfg(feature = "trace")]
#[test]
fn all_intermediates_match_release_javascript() {
    use zisk_solana_verifier::trace;
    let (guest, outputs, proof) = fixture();
    let actual = trace(&guest, &outputs, &proof).unwrap();
    let expected: Value = serde_json::from_str(include_str!("../../fixtures/trace.json")).unwrap();
    let scalar = |bytes: &[u8]| Fr::from_be_bytes_mod_order(bytes).into_bigint().to_string();
    assert_eq!(scalar(&actual.digest), expected["digest"].as_str().unwrap());
    for (key, values) in [
        ("challenges", actual.challenges.as_slice()),
        ("r", actual.r.as_slice()),
        ("quotients", actual.quotients.as_slice()),
    ] {
        for (i, value) in values.iter().enumerate() {
            assert_eq!(scalar(value), expected[key][i].as_str().unwrap(), "{key}[{i}]");
        }
    }
    assert_eq!(scalar(&actual.xi), expected["xi"].as_str().unwrap());
    assert_eq!(scalar(&actual.q), expected["q"].as_str().unwrap());
    for (key, point) in [("F", actual.f), ("E", actual.e), ("J", actual.j)] {
        for i in 0..2 {
            assert_eq!(
                Fq::from_be_bytes_mod_order(&point[i * 32..(i + 1) * 32]).into_bigint().to_string(),
                expected[key][i].as_str().unwrap(),
                "{key}[{i}]"
            );
        }
    }
}
