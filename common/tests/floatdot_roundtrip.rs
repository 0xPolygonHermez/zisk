//! Verify a bounded FloatDot blake3 proof against independently supplied expectations.
//!
//! Set FP32_PROOF, FP32_LEAF_SETUP_VK, FP32_PROGRAM_VK and FP32_MEASUREMENT to files,
//! then run `cargo test -p zisk-common --test floatdot_roundtrip -- --ignored --nocapture`.
//! Both key files are JSON arrays of four u64 limbs. Use the fresh setup's
//! vadcop_final.verkey.json and the ROM key generated independently from the ELF
//! whose SHA-256 is recorded in measurement.json; never extract keys from the proof.

use serde::Deserialize;
use std::{env, fs, path::PathBuf};
use zisk_common::{
    CommonError, HashMode, ProgramVK, Proof, ProofBody, PublicValues, VadcopKind, GOLDILOCKS_ORDER,
    PROGRAM_VK_LEN, ZISK_PUBLICS,
};

fn path(name: &str) -> PathBuf {
    env::var_os(name).unwrap_or_else(|| panic!("set {name} to its artifact path")).into()
}

fn key(name: &str) -> [u64; PROGRAM_VK_LEN] {
    let key: [u64; PROGRAM_VK_LEN] =
        serde_json::from_slice(&fs::read(path(name)).unwrap()).expect("key must be four JSON u64s");
    assert!(key.iter().all(|&word| word < GOLDILOCKS_ORDER), "{name} must be canonical");
    key
}

fn rejected(label: &str, result: Result<(), CommonError>) {
    assert!(matches!(result, Err(CommonError::NotVerified)), "{label}: {result:?}");
    println!("rejected {label}");
}

#[test]
#[ignore = "requires an actual bounded whole-VM proof and independently generated keys"]
fn verifies_floatdot_statement_and_rejects_tampering() {
    #[derive(Deserialize)]
    struct Measurement {
        expected_public_inputs_u32: Vec<u32>,
    }

    let leaf_vk = key("FP32_LEAF_SETUP_VK");
    let program_vk = ProgramVK { vk: key("FP32_PROGRAM_VK").to_vec(), hash_mode: HashMode::Blake3 };
    let measurement: Measurement =
        serde_json::from_slice(&fs::read(path("FP32_MEASUREMENT")).unwrap()).unwrap();
    assert_eq!(measurement.expected_public_inputs_u32.len(), ZISK_PUBLICS);
    let mut expected = program_vk.vk.clone();
    expected.extend(measurement.expected_public_inputs_u32.iter().map(|&v| u64::from(v)));
    let publics = PublicValues::new_from_u64(&expected);

    let proof = Proof::load(path("FP32_PROOF")).expect("load the actual exported proof");
    assert!(matches!(
        &proof.body,
        ProofBody::Vadcop { kind: VadcopKind::Final, hash, .. } if hash == "blake3"
    ));
    assert!(!proof.is_empty(), "execution or component checks are not a proof");
    assert_eq!(proof.publics_full().unwrap(), expected, "stored program and all 64 outputs");
    proof
        .with_leaf_setup_vk(&leaf_vk)
        .with_program_vk(&program_vk)
        .with_publics(&publics)
        .verify()
        .expect("verify the independently pinned FloatDot statement");
    println!("verified expected program and all 64 public outputs");

    // Change two public words separately, keeping both keys fixed.
    for output in [4, 9] {
        let mut altered = expected.clone();
        altered[PROGRAM_VK_LEN + output] ^= 1;
        let altered_publics = PublicValues::new_from_u64(&altered);
        rejected(
            &format!("public output {output} override"),
            proof
                .with_leaf_setup_vk(&leaf_vk)
                .with_program_vk(&program_vk)
                .with_publics(&altered_publics)
                .verify(),
        );

        let mut stored = proof.clone();
        let ProofBody::Vadcop { publics_full, .. } = &mut stored.body else { unreachable!() };
        *publics_full = altered;
        let stored = Proof::from_bytes(&stored.to_bytes().unwrap()).unwrap();
        rejected(
            &format!("serialized public output {output}"),
            stored.with_leaf_setup_vk(&leaf_vk).with_program_vk(&program_vk).verify(),
        );
    }

    let mut wrong_program = program_vk.clone();
    wrong_program.vk[0] = (wrong_program.vk[0] + 1) % GOLDILOCKS_ORDER;
    rejected(
        "wrong program key",
        proof
            .with_leaf_setup_vk(&leaf_vk)
            .with_program_vk(&wrong_program)
            .with_publics(&publics)
            .verify(),
    );

    let mut damaged = proof;
    let ProofBody::Vadcop { proof: payload, .. } = &mut damaged.body else { unreachable!() };
    payload[0] ^= 1;
    let damaged = Proof::from_bytes(&damaged.to_bytes().unwrap()).unwrap();
    rejected(
        "serialized proof payload",
        damaged
            .with_leaf_setup_vk(&leaf_vk)
            .with_program_vk(&program_vk)
            .with_publics(&publics)
            .verify(),
    );
}
