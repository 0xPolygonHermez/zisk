use zkvm_interface::{
    zkvm_bls12_381_fp2, zkvm_bls12_381_g2_point, zkvm_bls12_map_fp2_to_g2,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_bls12_map_fp2_to_g2() {
    // Fp2 = 0 + 0·i is canonical; smoke-test the G2 mapping.
    let fp2 = zkvm_bls12_381_fp2 { data: [0u8; 96] };
    let mut result = zkvm_bls12_381_g2_point { data: [0u8; 192] };
    let status = unsafe { zkvm_bls12_map_fp2_to_g2(&fp2, &mut result) };
    assert_eq!(status, ZKVM_EOK);
    assert!(result.data.iter().any(|&b| b != 0));
}
