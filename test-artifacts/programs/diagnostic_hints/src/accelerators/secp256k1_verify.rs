use zkvm_interface::{
    zkvm_secp256k1_hash, zkvm_secp256k1_pubkey, zkvm_secp256k1_signature, zkvm_secp256k1_verify,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

use super::limbs_to_be;

pub fn diagnostic_zkvm_secp256k1_verify() {
    // Valid (msg, sig, pk) tuple from test-artifacts/programs/secp256k1/src/ecdsa.rs,
    // expressed there as little-endian u64 limbs (limb[0] = LSB).
    let pk_x = [0x3bcfdc2aca47e0f2, 0xa739d5cc6b89e9b5, 0x35b73cc431afc6bc, 0xe1ea4273f638d4ae];
    let pk_y = [0xc6402318ee33448e, 0x9f18c242b8df8bb6, 0x934a8dfdd797e1c4, 0x3840aa9c4d86557e];
    let z = [0x1bf86a1816a52f52, 0xd31e26c3da73dda8, 0xa3b71997594da038, 0x17560495f6944673];
    let r = [0x68df7d8d7e0fb36b, 0xc2189fe681cd6e78, 0xc85ba1fd6238ecb5, 0x3e125456c8338994];
    let s = [0xd4e89d1ae75aeea2, 0xb8e33178783bd1a3, 0x866acebc9e141ec, 0x3a816b1c33739e41];

    let mut msg = zkvm_secp256k1_hash { data: [0u8; 32] };
    msg.data.copy_from_slice(&limbs_to_be(&z));
    let mut sig = zkvm_secp256k1_signature { data: [0u8; 64] };
    sig.data[0..32].copy_from_slice(&limbs_to_be(&r));
    sig.data[32..64].copy_from_slice(&limbs_to_be(&s));
    let mut pk = zkvm_secp256k1_pubkey { data: [0u8; 64] };
    pk.data[0..32].copy_from_slice(&limbs_to_be(&pk_x));
    pk.data[32..64].copy_from_slice(&limbs_to_be(&pk_y));

    let mut verified = false;
    let status = unsafe { zkvm_secp256k1_verify(&msg, &sig, &pk, &mut verified) };
    assert_eq!(status, ZKVM_EOK);
    assert!(verified);
}
