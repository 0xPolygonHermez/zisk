mod common;
use common::*;
use solana_address::Address;
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_rent::Rent;
use solana_signer::Signer;

fn consume(h: &Harness, target: Address, outputs: Vec<u8>) -> Instruction {
    Instruction {
        program_id: CONSUMER,
        accounts: vec![
            AccountMeta::new(h.receipt.pubkey(), true),
            AccountMeta::new_readonly(h.buffer.pubkey(), false),
            AccountMeta::new_readonly(target, false),
        ],
        data: outputs,
    }
}

#[test]
fn authenticated_cpi_updates_only_after_success() {
    let (_, outputs, proof) = fixture();
    let mut h = Harness::new();
    h.svm.add_program(CONSUMER, &elf("zisk_consumer_example")).unwrap();
    h.upload(&proof);
    let create = solana_system_interface::instruction::create_account(
        &h.payer.pubkey(),
        &h.receipt.pubkey(),
        h.svm.get_sysvar::<Rent>().minimum_balance(32),
        32,
        &CONSUMER,
    );
    h.send(vec![create], false).unwrap();
    let fake = Address::new_unique();
    h.svm.add_program(fake, &elf("zisk_test_noop")).unwrap();
    h.send(vec![Instruction { program_id: fake, accounts: vec![], data: vec![] }], false).unwrap();
    assert!(h.send(vec![consume(&h, fake, outputs.clone())], false).is_err());
    assert_eq!(h.svm.get_account(&h.receipt.pubkey()).unwrap().data, vec![0; 32]);
    let mut unsigned = consume(&h, ID, outputs.clone());
    unsigned.accounts[0].is_signer = false;
    assert!(h.send(vec![unsigned], false).is_err());
    let valid = h.send(vec![consume(&h, ID, outputs.clone())], false).unwrap();
    println!(
        "consumer valid CU={} headroom={}",
        valid.compute_units_consumed,
        1_400_000 - valid.compute_units_consumed
    );
    let good_state = h.svm.get_account(&h.receipt.pubkey()).unwrap().data;
    assert_eq!(good_state, solana_sha256_hasher::hash(&outputs).to_bytes());
    let mut account = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    let point = account.data[HEADER..HEADER + 64].to_vec();
    account.data[HEADER + 512..HEADER + 576].copy_from_slice(&point);
    h.svm.set_account(h.buffer.pubkey(), account).unwrap();
    let failure = h
        .send(vec![consume(&h, ID, outputs)], false)
        .expect_err("consumer ignored verification failure");
    assert_eq!(
        failure.err,
        solana_transaction_error::TransactionError::InstructionError(
            1,
            solana_instruction_error::InstructionError::Custom(3)
        )
    );
    assert_eq!(h.svm.get_account(&h.receipt.pubkey()).unwrap().data, good_state);
    println!("consumer invalid CU={}", failure.meta.compute_units_consumed);
}
