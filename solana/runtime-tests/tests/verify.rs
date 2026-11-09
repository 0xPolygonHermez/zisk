mod common;
use common::*;
use solana_address::Address;
use solana_instruction_error::InstructionError;
use solana_signer::Signer;
use solana_transaction_error::TransactionError;

#[test]
fn compiled_sbf_accepts_real_proof_and_rejects_late_mutation() {
    let (guest, outputs, proof) = fixture();
    for invalid in [false, true] {
        let mut h = Harness::new();
        let mut proof = proof.clone();
        if invalid {
            let point = proof[..64].to_vec();
            proof[512..576].copy_from_slice(&point);
        }
        h.upload(&proof);
        let result = h.verify(&guest, &outputs);
        let meta = if invalid {
            let fail = result.expect_err("invalid proof accepted");
            assert_eq!(
                fail.err,
                TransactionError::InstructionError(1, InstructionError::Custom(3))
            );
            fail.meta
        } else {
            result.expect("valid proof rejected")
        };
        println!("invalid={invalid} CU={} logs={:?}", meta.compute_units_consumed, meta.logs);
        assert!(meta.compute_units_consumed < 1_400_000);
    }
}

#[test]
fn rejects_wrong_owner_size_version_and_unsealed_data() {
    let (guest, outputs, proof) = fixture();
    let mut h = Harness::new();
    h.upload(&proof);
    let original = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    for case in 0..7 {
        let mut changed = original.clone();
        match case {
            0 => changed.owner = Address::new_unique(),
            1 => {
                changed.data.pop();
            }
            2 => changed.data.push(0),
            3 => changed.data[7] ^= 1,
            4 => changed.data[44] = 0,
            5 => changed.data[44] = 2,
            _ => changed.data[40..44].copy_from_slice(&u32::MAX.to_le_bytes()),
        }
        h.svm.set_account(h.buffer.pubkey(), changed).unwrap();
        assert!(h.verify(&guest, &outputs).is_err(), "case{case}");
    }
}

#[test]
fn every_proof_word_mutation_fails_in_sbf() {
    let (guest, outputs, proof) = fixture();
    let mut h = Harness::new();
    h.upload(&proof);
    let original = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    for word in 0..211 {
        let mut account = original.clone();
        account.data[HEADER + word * 32 + 31] ^= 1;
        h.svm.set_account(h.buffer.pubkey(), account).unwrap();
        let failure = h.verify(&guest, &outputs).expect_err("mutated proof accepted");
        assert!(
            matches!(
                failure.err,
                TransactionError::InstructionError(1, InstructionError::Custom(2 | 3 | 4))
            ),
            "word{word}: {:?}",
            failure.err
        );
        assert!(failure.meta.compute_units_consumed < 1_400_000);
    }
}
