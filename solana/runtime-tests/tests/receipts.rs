mod common;
use common::*;
use solana_address::{address, Address};
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_signer::Signer;

const LOADER: Address = address!("BPFLoaderUpgradeab1e11111111111111111111111");

fn record(h: &Harness, guest: &[u8], outputs: &[u8]) -> Instruction {
    Instruction {
        program_id: ID,
        accounts: vec![AccountMeta::new(h.buffer.pubkey(), false)],
        data: [vec![5], guest.to_vec(), outputs.to_vec()].concat(),
    }
}

fn digest(guest: &[u8], outputs: &[u8]) -> [u8; 32] {
    solana_sha256_hasher::hashv(&[b"ZisK:blake3-leaf:v1", guest, outputs]).to_bytes()
}

fn consume(h: &Harness, guest: &[u8], outputs: &[u8]) -> Instruction {
    let (receipt, _) =
        Address::find_program_address(&[b"consumed", &digest(guest, outputs)], &CONSUMER);
    let (programdata, _) = Address::find_program_address(&[ID.as_ref()], &LOADER);
    Instruction {
        program_id: CONSUMER,
        accounts: vec![
            AccountMeta::new(h.payer.pubkey(), true),
            AccountMeta::new(receipt, false),
            AccountMeta::new_readonly(h.buffer.pubkey(), false),
            AccountMeta::new_readonly(ID, false),
            AccountMeta::new_readonly(programdata, false),
            AccountMeta::new_readonly(address!("11111111111111111111111111111111"), false),
        ],
        data: [vec![1], outputs.to_vec()].concat(),
    }
}

#[test]
fn verified_statement_is_consumed_later_once_with_application_compute_available() {
    let (guest, outputs, proof) = fixture();
    let mut h = Harness::new();
    h.svm.add_program(CONSUMER, &elf("zisk_consumer_example")).unwrap();
    h.upload(&proof);
    let ix = consume(&h, &guest, &outputs);
    let receipt = ix.accounts[1].pubkey;
    h.verify(&guest, &outputs).unwrap();
    assert!(
        h.send(vec![ix.clone()], false).is_err(),
        "stateless verification cannot create a record"
    );
    let valid = h.send(vec![record(&h, &guest, &outputs)], false).unwrap();
    println!("record valid CU={}", valid.compute_units_consumed);
    let account = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    assert_eq!(account.data[44], 2);
    assert_eq!(&account.data[45..77], &digest(&guest, &outputs));
    let mut wrong_outputs = outputs.clone();
    wrong_outputs[0] ^= 1;
    assert!(h.send(vec![consume(&h, &guest, &wrong_outputs)], false).is_err());
    let mut unsigned = ix.clone();
    h.svm.airdrop(&h.receipt.pubkey(), 1_000_000).unwrap();
    unsigned.accounts[0].pubkey = h.receipt.pubkey();
    unsigned.accounts[0].is_signer = false;
    assert!(h.send(vec![unsigned], false).is_err());
    let mut wrong_receipt = ix.clone();
    wrong_receipt.accounts[1].pubkey = h.receipt.pubkey();
    assert!(h.send(vec![wrong_receipt], false).is_err());
    // Prefunding a predictable receipt must not prevent its legitimate creation.
    h.svm.airdrop(&receipt, 1_000_000).unwrap();
    let consumed = h.send(vec![ix.clone()], false).unwrap();
    assert!(
        consumed.compute_units_consumed < 50_000,
        "saved-result consumption exhausted application budget"
    );
    println!("consumer recorded CU={}", consumed.compute_units_consumed);
    assert_eq!(h.svm.get_account(&receipt).unwrap().data, digest(&guest, &outputs));
    assert!(h.send(vec![ix], false).is_err(), "same statement consumed twice");
    h.send(vec![h.instruction(vec![4])], false).unwrap();
    h.buffer = solana_keypair::Keypair::new();
    h.upload(&proof);
    h.send(vec![record(&h, &guest, &outputs)], false).unwrap();
    assert!(
        h.send(vec![consume(&h, &guest, &outputs)], false).is_err(),
        "new upload bypassed replay protection"
    );
}

#[test]
fn invalid_proof_never_records_and_close_clears_the_result() {
    let (guest, outputs, proof) = fixture();
    let mut h = Harness::new();
    h.upload(&proof);
    let original = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    let mut bad_guest = guest.clone();
    bad_guest[0] ^= 1;
    assert!(h.send(vec![record(&h, &bad_guest, &outputs)], false).is_err());
    assert_eq!(h.svm.get_account(&h.buffer.pubkey()).unwrap().data, original.data);
    let mut invalid = original.clone();
    invalid.data[HEADER + 512..HEADER + 576].copy_from_slice(&proof[..64]);
    h.svm.set_account(h.buffer.pubkey(), invalid.clone()).unwrap();
    assert!(h.send(vec![record(&h, &guest, &outputs)], false).is_err());
    assert_eq!(h.svm.get_account(&h.buffer.pubkey()).unwrap().data, invalid.data);
    h.svm.set_account(h.buffer.pubkey(), original).unwrap();
    let mut readonly = record(&h, &guest, &outputs);
    readonly.accounts[0].is_writable = false;
    assert!(h.send(vec![readonly], false).is_err());
    h.send(vec![record(&h, &guest, &outputs)], false).unwrap();
    h.send(vec![h.instruction(vec![4])], false).unwrap();
    h.create();
    let fresh = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    assert_eq!(fresh.data[44], 0);
    assert!(fresh.data[45..77].iter().all(|v| *v == 0));
    assert!(h.send(vec![record(&h, &guest, &outputs)], false).is_err());
}

#[test]
fn consumer_rejects_upgrade_authority_and_substituted_program_data() {
    let (guest, outputs, proof) = fixture();
    let mut h = Harness::new();
    h.svm.add_program(CONSUMER, &elf("zisk_consumer_example")).unwrap();
    h.upload(&proof);
    h.send(vec![record(&h, &guest, &outputs)], false).unwrap();
    let ix = consume(&h, &guest, &outputs);
    let data_key = ix.accounts[4].pubkey;
    let original = h.svm.get_account(&data_key).unwrap();
    let mut upgradeable = original.clone();
    upgradeable.data[12] = 1;
    upgradeable.data[13..45].copy_from_slice(h.payer.pubkey().as_ref());
    h.svm.set_account(data_key, upgradeable).unwrap();
    assert!(h.send(vec![ix.clone()], false).is_err(), "upgradeable verifier trusted");
    h.svm.set_account(data_key, original.clone()).unwrap();
    let mut substituted = ix.clone();
    let fake = Address::new_unique();
    h.svm.set_account(fake, original).unwrap();
    substituted.accounts[4].pubkey = fake;
    assert!(h.send(vec![substituted], false).is_err(), "unrelated immutable metadata trusted");
    assert!(h.svm.get_account(&ix.accounts[1].pubkey).is_none());
    h.send(vec![ix], false).unwrap();
}
