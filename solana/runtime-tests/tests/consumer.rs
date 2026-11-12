mod common;
use common::*;
use solana_address::Address;
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_signer::Signer;

fn consume(h: &Harness, target: Address, outputs: Vec<u8>) -> Instruction {
    let digest =
        solana_sha256_hasher::hashv(&[b"ZisK:blake3-leaf:v1", &fixture().0, &outputs]).to_bytes();
    let receipt = Address::find_program_address(&[b"consumed", &digest], &CONSUMER).0;
    let loader = solana_address::address!("BPFLoaderUpgradeab1e11111111111111111111111");
    let programdata = Address::find_program_address(&[target.as_ref()], &loader).0;
    Instruction {
        program_id: CONSUMER,
        accounts: vec![
            AccountMeta::new(h.payer.pubkey(), true),
            AccountMeta::new(receipt, false),
            AccountMeta::new_readonly(h.buffer.pubkey(), false),
            AccountMeta::new_readonly(target, false),
            AccountMeta::new_readonly(programdata, false),
            AccountMeta::new_readonly(
                solana_address::address!("11111111111111111111111111111111"),
                false,
            ),
        ],
        data: [vec![0], outputs].concat(),
    }
}

#[test]
fn authenticated_cpi_updates_only_after_success() {
    let (_, outputs, proof) = fixture();
    let mut h = Harness::new();
    h.svm.add_program(CONSUMER, &elf("zisk_consumer_example")).unwrap();
    h.upload(&proof);
    let receipt = consume(&h, ID, outputs.clone()).accounts[1].pubkey;
    let fake = Address::new_unique();
    h.svm.add_program(fake, &elf("zisk_test_noop")).unwrap();
    h.send(vec![Instruction { program_id: fake, accounts: vec![], data: vec![] }], false).unwrap();
    assert!(h.send(vec![consume(&h, fake, outputs.clone())], false).is_err());
    assert!(h.svm.get_account(&receipt).is_none());
    let mut unsigned = consume(&h, ID, outputs.clone());
    h.svm.airdrop(&h.receipt.pubkey(), 1_000_000).unwrap();
    unsigned.accounts[0].pubkey = h.receipt.pubkey();
    unsigned.accounts[0].is_signer = false;
    assert!(h.send(vec![unsigned], false).is_err());
    let mut account = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    let original = account.clone();
    let point = account.data[HEADER..HEADER + 64].to_vec();
    account.data[HEADER + 512..HEADER + 576].copy_from_slice(&point);
    h.svm.set_account(h.buffer.pubkey(), account).unwrap();
    let failure = h
        .send(vec![consume(&h, ID, outputs.clone())], false)
        .expect_err("consumer ignored verification failure");
    assert_eq!(
        failure.err,
        solana_transaction_error::TransactionError::InstructionError(
            1,
            solana_instruction_error::InstructionError::Custom(3)
        )
    );
    assert!(h.svm.get_account(&receipt).is_none());
    println!("consumer invalid CU={}", failure.meta.compute_units_consumed);
    h.svm.set_account(h.buffer.pubkey(), original).unwrap();
    let valid = h.send(vec![consume(&h, ID, outputs.clone())], false).unwrap();
    println!(
        "consumer valid CU={} headroom={}",
        valid.compute_units_consumed,
        1_400_000 - valid.compute_units_consumed
    );
    assert_eq!(
        h.svm.get_account(&receipt).unwrap().data,
        solana_sha256_hasher::hashv(&[b"ZisK:blake3-leaf:v1", &fixture().0, &outputs]).to_bytes()
    );
    assert!(h.send(vec![consume(&h, ID, outputs)], false).is_err());
}
