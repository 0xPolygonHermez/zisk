#![no_main]
ziskos::entrypoint!(main);

// Matched secp256k1 ECDSA-verify A/B (identical signature — a plain rename).
#[cfg(not(feature = "ziskasm"))]
use ziskos::zisklib::ecdsa_verify_secp256k1;
#[cfg(feature = "ziskasm")]
use zisklib::secp256k1_ecdsa_verify as ecdsa_verify_secp256k1;

fn main() {
    // Known-good vector (test-artifacts secp256k1 ecdsa test).
    let pk = [
        0x3bcfdc2aca47e0f2, 0xa739d5cc6b89e9b5, 0x35b73cc431afc6bc, 0xe1ea4273f638d4ae,
        0xc6402318ee33448e, 0x9f18c242b8df8bb6, 0x934a8dfdd797e1c4, 0x3840aa9c4d86557e,
    ];
    let z = [0x1bf86a1816a52f52, 0xd31e26c3da73dda8, 0xa3b71997594da038, 0x17560495f6944673];
    let r = [0x68df7d8d7e0fb36b, 0xc2189fe681cd6e78, 0xc85ba1fd6238ecb5, 0x3e125456c8338994];
    let s = [0xd4e89d1ae75aeea2, 0xb8e33178783bd1a3, 0x866acebc9e141ec, 0x3a816b1c33739e41];

    let mut ok = 0u64;
    for _ in 0..20 {
        if ecdsa_verify_secp256k1(&pk, &z, &r, &s) {
            ok += 1;
        }
    }
    println!("secp256k1_ab ok={}", ok);
}
