#![no_main]
ziskos::entrypoint!(main);

// Matched BLS12-381 hash-to-curve-G2 A/B (identical signature — a rename).
#[cfg(not(feature = "ziskasm"))]
use ziskos::zisklib::hash_to_curve_g2_bls12_381;
#[cfg(feature = "ziskasm")]
use zisklib::bls12_381_hash_to_curve_g2 as hash_to_curve_g2_bls12_381;

fn main() {
    let dst = b"QUUX-V01-CS02-with-BLS12381G2_XMD:SHA-256_SSWU_RO_";
    let mut acc = 0u64;
    for i in 0u8..5 {
        let msg = [b'a', b'b', b'c', i];
        let p = hash_to_curve_g2_bls12_381(&msg, dst);
        for l in p.iter() {
            acc ^= *l;
        }
    }
    println!("bls_ab acc={:016x}", acc);
}
