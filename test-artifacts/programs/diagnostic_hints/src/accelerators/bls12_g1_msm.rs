use zisk_zkvm_interface::{
    zkvm_bls12_381_g1_msm_pair, zkvm_bls12_381_g1_point, zkvm_bls12_g1_msm,
    zkvm_status_ZKVM_EFAIL as ZKVM_EFAIL,
};

pub fn diagnostic_zkvm_bls12_g1_msm() {
    // EIP-2537 has no empty G1 MSM: a zero-length input is rejected outright rather
    // than reported as ∞, so the result is left untouched. The 0xff fill is a
    // sentinel -- it must survive the call.
    let pairs: [zkvm_bls12_381_g1_msm_pair; 0] = [];
    let mut result = zkvm_bls12_381_g1_point { data: [0xffu8; 96] };
    let status = unsafe { zkvm_bls12_g1_msm(pairs.as_ptr(), 0, &mut result) };
    assert_eq!(status, ZKVM_EFAIL);
    assert_eq!(result.data, [0xffu8; 96]);
}
