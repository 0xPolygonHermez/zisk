use zkvm_interface::{
    zkvm_bn254_g1_mul, zkvm_bn254_g1_point, zkvm_bn254_scalar, zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_bn254_g1_mul() {
    // G * 1 = G, where the BN254 G1 generator is (x = 1, y = 2).
    let mut point = zkvm_bn254_g1_point { data: [0u8; 64] };
    point.data[31] = 1;
    point.data[63] = 2;
    let mut scalar = zkvm_bn254_scalar { data: [0u8; 32] };
    scalar.data[31] = 1;
    let mut result = zkvm_bn254_g1_point { data: [0xffu8; 64] };
    let status = unsafe { zkvm_bn254_g1_mul(&point, &scalar, &mut result) };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(result.data, point.data);
}
