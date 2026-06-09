use zkvm_interface::{
    zkvm_bls12_381_pairing_pair, zkvm_bls12_pairing, zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_bls12_pairing() {
    // Empty product of pairings is the identity element, so verification passes.
    let pairs: [zkvm_bls12_381_pairing_pair; 0] = [];
    let mut verified = false;
    let status = unsafe { zkvm_bls12_pairing(pairs.as_ptr(), 0, &mut verified) };
    assert_eq!(status, ZKVM_EOK);
    assert!(verified);
}
