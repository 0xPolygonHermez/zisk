use zkvm_interface::{zkvm_bn254_g1_add, zkvm_bn254_g1_point, zkvm_status_ZKVM_EOK as ZKVM_EOK};

pub fn diagnostic_zkvm_bn254_g1_add() {
    // EIP-196 encodes the point at infinity as (x, y) = (0, 0). ∞ + ∞ = ∞.
    let p1 = zkvm_bn254_g1_point { data: [0u8; 64] };
    let p2 = zkvm_bn254_g1_point { data: [0u8; 64] };
    let mut result = zkvm_bn254_g1_point { data: [0xffu8; 64] };
    let status = unsafe { zkvm_bn254_g1_add(&p1, &p2, &mut result) };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(result.data, [0u8; 64]);
}
