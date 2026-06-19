use zkvm_interface::{
    zkvm_secp256k1_ecrecover, zkvm_secp256k1_hash, zkvm_secp256k1_pubkey, zkvm_secp256k1_signature,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

use super::limbs_to_be;

pub fn diagnostic_zkvm_secp256k1_ecrecover() {
    // Same valid (msg, sig, pk) tuple as zkvm_secp256k1_verify. The recovery id encodes
    // the parity of the nonce point k·G (not of pk_y), so we try both candidates and
    // require that one recovers the original pubkey.
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
    let mut expected = [0u8; 64];
    expected[0..32].copy_from_slice(&limbs_to_be(&pk_x));
    expected[32..64].copy_from_slice(&limbs_to_be(&pk_y));

    let mut matched = false;
    for recid in [0u8, 1u8] {
        let mut output = zkvm_secp256k1_pubkey { data: [0u8; 64] };
        let status = unsafe { zkvm_secp256k1_ecrecover(&msg, &sig, recid, &mut output) };
        if status == ZKVM_EOK && output.data == expected {
            matched = true;
            break;
        }
    }
    assert!(matched, "ecrecover did not recover the expected pubkey for any recid");
}
