use zkvm_interface::{
    zkvm_bls12_381_g1_msm_pair, zkvm_bls12_381_g1_point, zkvm_bls12_g1_msm,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_bls12_g1_msm() {
    // Empty MSM yields ∞ (the identity element).
    let pairs: [zkvm_bls12_381_g1_msm_pair; 0] = [];
    let mut result = zkvm_bls12_381_g1_point { data: [0xffu8; 96] };
    let status = unsafe { zkvm_bls12_g1_msm(pairs.as_ptr(), 0, &mut result) };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(result.data, [0u8; 96]);
}
