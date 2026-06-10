use zkvm_interface::{
    zkvm_bls12_381_fp, zkvm_bls12_381_g1_point, zkvm_bls12_map_fp_to_g1,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_bls12_map_fp_to_g1() {
    // Fp = 0 is a canonical field element; smoke-test that the SWU map succeeds and
    // produces a non-infinity result.
    let fp = zkvm_bls12_381_fp { data: [0u8; 48] };
    let mut result = zkvm_bls12_381_g1_point { data: [0u8; 96] };
    let status = unsafe { zkvm_bls12_map_fp_to_g1(&fp, &mut result) };
    assert_eq!(status, ZKVM_EOK);
    assert!(result.data.iter().any(|&b| b != 0));
}
