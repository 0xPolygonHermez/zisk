//! Baseline cost of Ed25519 verification on ZisK with NO ed25519/sha512 acceleration,
//! split into its parts, plus calibration points that are accelerated today.
//!
//! Each region is wrapped in a profile tag; `ziskemu -e <elf> -X` prints `[tag] cost` per region.
//! Vectors in `vectors.rs` come from `scripts/gen_vectors.py`.
#![no_main]
ziskos::entrypoint!(main);

mod vectors;
use vectors::*;

use core::hint::black_box;
use curve25519_dalek::{edwards::CompressedEdwardsY, EdwardsPoint, Scalar};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256, Sha512};

fn msg(n: usize) -> Vec<u8> {
    (0..n).map(|i| ((i * 7 + 3) % 251) as u8).collect()
}

macro_rules! bench_size {
    ($n:literal, $sig:ident, $verify:ident, $verify_strict:ident, $hash:ident, $sha256_sw:ident, $sha256_hw:ident) => {{
        let m = msg($n);
        let sig = Signature::from_bytes(&$sig);
        let vk = VerifyingKey::from_bytes(&ED_PK).unwrap();

        ziskos::profile_start!($verify);
        let ok = black_box(&vk).verify(black_box(&m), black_box(&sig)).is_ok();
        ziskos::profile_end!($verify);
        assert!(ok);

        ziskos::profile_start!($verify_strict);
        let ok = black_box(&vk).verify_strict(black_box(&m), black_box(&sig)).is_ok();
        ziskos::profile_end!($verify_strict);
        assert!(ok);

        // k = SHA-512(R || A || M) mod L
        ziskos::profile_start!($hash);
        let mut h = Sha512::new();
        h.update(&$sig[..32]);
        h.update(&ED_PK);
        h.update(black_box(&m));
        let k = Scalar::from_hash(h);
        ziskos::profile_end!($hash);
        black_box(k);

        // Calibration: same message through software SHA-256 vs zisklib (sha256f-accelerated).
        ziskos::profile_start!($sha256_sw);
        let a: [u8; 32] = Sha256::digest(black_box(&m)).into();
        ziskos::profile_end!($sha256_sw);
        ziskos::profile_start!($sha256_hw);
        let b = ziskos::zisklib::sha256(black_box(&m));
        ziskos::profile_end!($sha256_hw);
        assert_eq!(a, b);
    }};
}

fn main() {
    bench_size!(32, ED_SIG_32, verify_32, verify_strict_32, hash_32, sha256_sw_32, sha256_hw_32);
    bench_size!(
        256,
        ED_SIG_256,
        verify_256,
        verify_strict_256,
        hash_256,
        sha256_sw_256,
        sha256_hw_256
    );
    bench_size!(
        1232,
        ED_SIG_1232,
        verify_1232,
        verify_strict_1232,
        hash_1232,
        sha256_sw_1232,
        sha256_hw_1232
    );
    bench_size!(
        4096,
        ED_SIG_4096,
        verify_4096,
        verify_strict_4096,
        hash_4096,
        sha256_sw_4096,
        sha256_hw_4096
    );

    // Curve-side pieces (message-size independent), using the 32-byte vector.
    let m = msg(32);
    let r_bytes: [u8; 32] = ED_SIG_32[..32].try_into().unwrap();
    let s = Scalar::from_canonical_bytes(ED_SIG_32[32..].try_into().unwrap()).unwrap();
    let mut h = Sha512::new();
    h.update(&r_bytes);
    h.update(&ED_PK);
    h.update(&m);
    let k = Scalar::from_hash(h);

    ziskos::profile_start!(decompress_a);
    let a = CompressedEdwardsY(black_box(ED_PK)).decompress().unwrap();
    ziskos::profile_end!(decompress_a);

    let neg_a = -a;
    ziskos::profile_start!(double_scalar_mul);
    let r = EdwardsPoint::vartime_double_scalar_mul_basepoint(
        black_box(&k),
        black_box(&neg_a),
        black_box(&s),
    );
    ziskos::profile_end!(double_scalar_mul);

    ziskos::profile_start!(compress_r);
    let rc = black_box(r).compress();
    ziskos::profile_end!(compress_r);
    assert_eq!(rc.to_bytes(), r_bytes);

    ziskos::profile_start!(mul_base_ct);
    let p = EdwardsPoint::mul_base(black_box(&k));
    ziskos::profile_end!(mul_base_ct);
    black_box(p);

    // Calibration: secp256k1 ECDSA verify through zisklib (secp256k1 add/dbl + arith256_mod accelerated).
    let be = ziskos::zisklib::be_bytes_to_u64_4;
    let (px, py) = (be(&K1_PKX), be(&K1_PKY));
    let pk = [px[0], px[1], px[2], px[3], py[0], py[1], py[2], py[3]];
    let (z, r1, s1) = (be(&K1_Z), be(&K1_R), be(&K1_S));
    ziskos::profile_start!(secp256k1_ecdsa_verify_accel);
    let ok = ziskos::zisklib::ecdsa_verify_secp256k1(black_box(&pk), &z, &r1, &s1);
    ziskos::profile_end!(secp256k1_ecdsa_verify_accel);
    assert!(ok);
}
