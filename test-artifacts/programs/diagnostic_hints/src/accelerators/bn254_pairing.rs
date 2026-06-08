use zkvm_interface::{
    zkvm_bn254_pairing, zkvm_bn254_pairing_pair, zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_bn254_pairing() {
    // Empty product of pairings is the identity element, so verification passes.
    let pairs: [zkvm_bn254_pairing_pair; 0] = [];
    let mut verified = false;
    let status = unsafe { zkvm_bn254_pairing(pairs.as_ptr(), 0, &mut verified) };
    assert_eq!(status, ZKVM_EOK);
    assert!(verified);
}
