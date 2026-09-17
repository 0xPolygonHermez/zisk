use zisk_zkvm_interface::{
    zkvm_bls12_381_pairing_pair, zkvm_bls12_pairing, zkvm_status_ZKVM_EFAIL as ZKVM_EFAIL,
};

pub fn diagnostic_zkvm_bls12_pairing() {
    // Unlike EIP-197 on bn254, EIP-2537 has no empty pairing product: ZisK requires
    // 1..=32 pairs, so a zero-length input is rejected instead of verifying as the
    // identity. `verified` is left untouched -- seeded true so the assert below
    // catches a write as well as a wrong value.
    let pairs: [zkvm_bls12_381_pairing_pair; 0] = [];
    let mut verified = true;
    let status = unsafe { zkvm_bls12_pairing(pairs.as_ptr(), 0, &mut verified) };
    assert_eq!(status, ZKVM_EFAIL);
    assert!(verified);
}
