mod blake2f;
mod bls12_g1_add;
mod bls12_g1_msm;
mod bls12_g2_add;
mod bls12_g2_msm;
mod bls12_map_fp2_to_g2;
mod bls12_map_fp_to_g1;
mod bls12_pairing;
mod bn254_g1_add;
mod bn254_g1_mul;
mod bn254_pairing;
mod keccak256;
mod kzg_point_eval;
mod modexp;
mod ripemd160;
mod secp256k1_ecrecover;
mod secp256k1_verify;
mod secp256r1_verify;
mod sha256;

pub fn diagnostic_accelerators() {
    keccak256::diagnostic_zkvm_keccak256();
    sha256::diagnostic_zkvm_sha256();
    ripemd160::diagnostic_zkvm_ripemd160();
    modexp::diagnostic_zkvm_modexp();
    bn254_g1_add::diagnostic_zkvm_bn254_g1_add();
    bn254_g1_mul::diagnostic_zkvm_bn254_g1_mul();
    bn254_pairing::diagnostic_zkvm_bn254_pairing();
    blake2f::diagnostic_zkvm_blake2f();
    kzg_point_eval::diagnostic_zkvm_kzg_point_eval();
    bls12_g1_add::diagnostic_zkvm_bls12_g1_add();
    bls12_g1_msm::diagnostic_zkvm_bls12_g1_msm();
    bls12_g2_add::diagnostic_zkvm_bls12_g2_add();
    bls12_g2_msm::diagnostic_zkvm_bls12_g2_msm();
    bls12_pairing::diagnostic_zkvm_bls12_pairing();
    bls12_map_fp_to_g1::diagnostic_zkvm_bls12_map_fp_to_g1();
    bls12_map_fp2_to_g2::diagnostic_zkvm_bls12_map_fp2_to_g2();
    secp256k1_verify::diagnostic_zkvm_secp256k1_verify();
    secp256k1_ecrecover::diagnostic_zkvm_secp256k1_ecrecover();
    secp256r1_verify::diagnostic_zkvm_secp256r1_verify();

    println!("All accelerator diagnostics passed!");
}

/// Convert a 256-bit value stored as 4 little-endian u64 limbs (limb[0] = LSB)
/// into 32 big-endian bytes — the format expected by the zkvm accelerator API.
fn limbs_to_be(limbs: &[u64; 4]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[0..8].copy_from_slice(&limbs[3].to_be_bytes());
    out[8..16].copy_from_slice(&limbs[2].to_be_bytes());
    out[16..24].copy_from_slice(&limbs[1].to_be_bytes());
    out[24..32].copy_from_slice(&limbs[0].to_be_bytes());
    out
}
