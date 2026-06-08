use zkvm_interface::{
    zkvm_secp256r1_hash, zkvm_secp256r1_pubkey, zkvm_secp256r1_signature, zkvm_secp256r1_verify,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

use super::limbs_to_be;

pub fn diagnostic_zkvm_secp256r1_verify() {
    // Valid (msg, sig, pk) tuple from test-artifacts/programs/secp256r1/src/ecdsa.rs
    // (originally from go-ethereum's p256Verify.json), as little-endian u64 limbs.
    let pk_x = [0x69c8c4df6c732838, 0x2903269919f70860, 0xdcfe467828128bad, 0x2927b10512bae3ed];
    let pk_y = [0x8d1a974e7341513e, 0x6766b3d968500155, 0x921fb1498a60f460, 0xc7787964eaac00e5];
    let z = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s = [0x77a67f79e6fadd76, 0x525fe710fab9aa7c, 0x3c7b11eb6c4e0ae7, 0x4cd60b855d442f5b];

    let mut msg = zkvm_secp256r1_hash { data: [0u8; 32] };
    msg.data.copy_from_slice(&limbs_to_be(&z));
    let mut sig = zkvm_secp256r1_signature { data: [0u8; 64] };
    sig.data[0..32].copy_from_slice(&limbs_to_be(&r));
    sig.data[32..64].copy_from_slice(&limbs_to_be(&s));
    let mut pk = zkvm_secp256r1_pubkey { data: [0u8; 64] };
    pk.data[0..32].copy_from_slice(&limbs_to_be(&pk_x));
    pk.data[32..64].copy_from_slice(&limbs_to_be(&pk_y));

    let mut verified = false;
    let status = unsafe { zkvm_secp256r1_verify(&msg, &sig, &pk, &mut verified) };
    assert_eq!(status, ZKVM_EOK);
    assert!(verified);
}
