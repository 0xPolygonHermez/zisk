use zkvm_interface::{
    zkvm_kzg_commitment, zkvm_kzg_field_element, zkvm_kzg_point_eval, zkvm_kzg_proof,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_kzg_point_eval() {
    // Trivial instance: polynomial f(x) = 0 evaluated at z = 0 yields y = 0, with both
    // commitment and proof equal to the point at infinity (compressed encoding of ∞ in
    // G1 is 0xc0 followed by 47 zero bytes).
    let mut commitment = zkvm_kzg_commitment { data: [0u8; 48] };
    commitment.data[0] = 0xc0;
    let z = zkvm_kzg_field_element { data: [0u8; 32] };
    let y = zkvm_kzg_field_element { data: [0u8; 32] };
    let mut proof = zkvm_kzg_proof { data: [0u8; 48] };
    proof.data[0] = 0xc0;
    let mut verified = false;
    let status = unsafe { zkvm_kzg_point_eval(&commitment, &z, &y, &proof, &mut verified) };
    assert_eq!(status, ZKVM_EOK);
    assert!(verified);
}
