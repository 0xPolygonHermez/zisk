use zkvm_interface::{
    zkvm_bls12_381_g2_point, zkvm_bls12_g2_add, zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_bls12_g2_add() {
    // ∞ + ∞ = ∞ on G2.
    let p1 = zkvm_bls12_381_g2_point { data: [0u8; 192] };
    let p2 = zkvm_bls12_381_g2_point { data: [0u8; 192] };
    let mut result = zkvm_bls12_381_g2_point { data: [0xffu8; 192] };
    let status = unsafe { zkvm_bls12_g2_add(&p1, &p2, &mut result) };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(result.data, [0u8; 192]);
}
