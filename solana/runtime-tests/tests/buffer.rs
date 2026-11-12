mod common;
use common::*;
use solana_address::Address;
use solana_signer::Signer;

#[test]
fn upload_lifecycle_rejects_invalid_writes_and_returns_rent() {
    let (_, _, proof) = fixture();
    let mut h = Harness::new();
    let create_cu = h.create();
    assert!(h.seal().is_err());
    h.svm.airdrop(&h.receipt.pubkey(), 1_000_000).unwrap();
    for operation in [2, 4] {
        let mut wrong = h.instruction(vec![operation]);
        wrong.accounts[1].pubkey = h.receipt.pubkey();
        let failure = h.send(vec![wrong], false).unwrap_err();
        assert_eq!(
            failure.err,
            solana_transaction_error::TransactionError::InstructionError(
                1,
                solana_instruction_error::InstructionError::IllegalOwner
            )
        );
    }

    assert!(h.append(1, &[1]).is_err());
    assert!(h.append(0, &[]).is_err());
    assert!(h.append(0, &vec![1; 901]).is_err());
    let mut wrong = h.instruction(vec![2]);
    wrong.accounts[1].pubkey = Address::new_unique();
    wrong.accounts[1].is_signer = false;
    assert!(h.send(vec![wrong], false).is_err());
    let mut append_cu = Vec::new();
    for (i, chunk) in proof.chunks(900).enumerate() {
        append_cu.push(h.append(i * 900, chunk).unwrap().compute_units_consumed);
    }
    assert!(h.append(0, &[1]).is_err());
    assert!(h.append(PROOF, &[1]).is_err());
    let seal_cu = h.seal().unwrap().compute_units_consumed;
    assert!(h.seal().is_err());
    assert!(h.append(PROOF, &[1]).is_err());
    let account = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    assert_eq!(&account.data[HEADER..], &proof);
    assert_eq!(account.data[44], 1);
    let before = h.svm.get_balance(&h.payer.pubkey()).unwrap();
    let close_cu = h.send(vec![h.instruction(vec![4])], false).unwrap().compute_units_consumed;
    assert_eq!(h.svm.get_balance(&h.payer.pubkey()).unwrap(), before + account.lamports - 5000);
    assert!(h.svm.get_account(&h.buffer.pubkey()).is_none());
    // A closed address can be created again only with its keypair; the previous seal is gone.
    h.create();
    assert!(h.verify(&fixture().0, &fixture().1).is_err());
    let mut alias = h.instruction(vec![4]);
    alias.accounts[1].pubkey = h.buffer.pubkey();
    let before_alias = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    assert!(h.send(vec![alias], true).is_err());
    let after_alias = h.svm.get_account(&h.buffer.pubkey()).unwrap();
    assert_eq!(before_alias.data, after_alias.data);
    assert_eq!(before_alias.lamports, after_alias.lamports);
    println!(
        "upload chunks=8 bytes={} rent={} largest_tx={}",
        PROOF,
        account.lamports,
        h.sizes.iter().max().unwrap()
    );
    println!("buffer CU: create={create_cu} append={append_cu:?} seal={seal_cu} close={close_cu}");
}

#[test]
fn instruction_boundaries_fail_without_panicking() {
    let (_, _, proof) = fixture();
    let mut h = Harness::new();
    h.upload(&proof);
    for data in [
        vec![],
        vec![255],
        vec![0],
        vec![1],
        vec![1, 255, 255, 255, 255, 1],
        vec![2, 0],
        vec![3],
        vec![3; 546],
        vec![4, 0],
    ] {
        let mut ix = h.instruction(if data.is_empty() { vec![3] } else { data.clone() });
        ix.data = data;
        let sign_buffer = ix.accounts[0].is_signer;
        assert!(h.send(vec![ix], sign_buffer).is_err());
    }
}
