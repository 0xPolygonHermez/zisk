use zkvm_interface::{
    zkvm_bls12_381_g1_point, zkvm_bls12_g1_add, zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_bls12_g1_add() {
    // EIP-2537 encodes ∞ as the all-zero G1 point. ∞ + ∞ = ∞.
    let p1 = zkvm_bls12_381_g1_point { data: [0u8; 96] };
    let p2 = zkvm_bls12_381_g1_point { data: [0u8; 96] };
    let mut result = zkvm_bls12_381_g1_point { data: [0xffu8; 96] };
    let status = unsafe { zkvm_bls12_g1_add(&p1, &p2, &mut result) };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(result.data, [0u8; 96]);
}
