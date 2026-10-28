//! Cost of Ed25519 verification on ZisK: `ed25519-dalek` in software (the baseline) against
//! zisklib's verifier on the `ed25519_add` precompile, split into its parts. Both verifiers must
//! agree on every input (valid and tampered).
//!
//! Each region is wrapped in a profile tag, compiled in only with `--features profile`; then
//! `ziskemu -e <elf> -X` prints `[tag] cost` per region. Profile tags are emulator only (Main's
//! profile opcode is not proven), so the default build is the one to verify constraints or prove.
//! Vectors in `vectors.rs` come from `scripts/gen_vectors.py`.
#![no_main]
ziskos::entrypoint!(main);

mod vectors;
use vectors::*;

use core::hint::black_box;
use curve25519_dalek::{edwards::CompressedEdwardsY, EdwardsPoint, Scalar};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha512};

/// Profile tags, only with `--features profile` (see the module docs).
macro_rules! tag_start {
    ($name:ident) => {
        #[cfg(feature = "profile")]
        ziskos::profile_start!($name);
    };
}

macro_rules! tag_end {
    ($name:ident) => {
        #[cfg(feature = "profile")]
        ziskos::profile_end!($name);
    };
}

fn msg(n: usize) -> Vec<u8> {
    (0..n).map(|i| ((i * 7 + 3) % 251) as u8).collect()
}

macro_rules! bench_size {
    ($n:literal, $sig:ident, $verify:ident, $verify_strict:ident, $verify_zisk:ident, $verify_strict_zisk:ident, $hash:ident $(,)?) => {{
        let m = msg($n);
        let sig = Signature::from_bytes(&$sig);
        let vk = VerifyingKey::from_bytes(&ED_PK).unwrap();

        tag_start!($verify);
        let ok = black_box(&vk).verify(black_box(&m), black_box(&sig)).is_ok();
        tag_end!($verify);
        assert!(ok);

        tag_start!($verify_zisk);
        let ok =
            ziskos::zisklib::ed25519_verify(black_box(&ED_PK), black_box(&m), black_box(&$sig));
        tag_end!($verify_zisk);
        assert!(ok);

        tag_start!($verify_strict);
        let ok = black_box(&vk).verify_strict(black_box(&m), black_box(&sig)).is_ok();
        tag_end!($verify_strict);
        assert!(ok);

        tag_start!($verify_strict_zisk);
        let ok = ziskos::zisklib::ed25519_verify_strict(
            black_box(&ED_PK),
            black_box(&m),
            black_box(&$sig),
        );
        tag_end!($verify_strict_zisk);
        assert!(ok);

        // k = SHA-512(R || A || M) mod L
        tag_start!($hash);
        let mut h = Sha512::new();
        h.update(&$sig[..32]);
        h.update(&ED_PK);
        h.update(black_box(&m));
        let k = Scalar::from_hash(h);
        tag_end!($hash);
        black_box(k);
    }};
}

fn main() {
    bench_size!(
        32,
        ED_SIG_32,
        verify_32,
        verify_strict_32,
        verify_zisk_32,
        verify_strict_zisk_32,
        hash_32,
    );
    bench_size!(
        256,
        ED_SIG_256,
        verify_256,
        verify_strict_256,
        verify_zisk_256,
        verify_strict_zisk_256,
        hash_256,
    );
    bench_size!(
        1232,
        ED_SIG_1232,
        verify_1232,
        verify_strict_1232,
        verify_zisk_1232,
        verify_strict_zisk_1232,
        hash_1232,
    );
    bench_size!(
        4096,
        ED_SIG_4096,
        verify_4096,
        verify_strict_4096,
        verify_zisk_4096,
        verify_strict_zisk_4096,
        hash_4096,
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

    tag_start!(decompress_a);
    let a = CompressedEdwardsY(black_box(ED_PK)).decompress().unwrap();
    tag_end!(decompress_a);

    tag_start!(decompress_a_zisk);
    let a_zisk = ziskos::zisklib::decompress_ed25519(black_box(&ED_PK)).unwrap();
    tag_end!(decompress_a_zisk);

    let neg_a = -a;
    tag_start!(double_scalar_mul);
    let r = EdwardsPoint::vartime_double_scalar_mul_basepoint(
        black_box(&k),
        black_box(&neg_a),
        black_box(&s),
    );
    tag_end!(double_scalar_mul);

    tag_start!(compress_r);
    let rc = black_box(r).compress();
    tag_end!(compress_r);
    assert_eq!(rc.to_bytes(), r_bytes);

    let limbs = |x: &Scalar| -> [u64; 4] {
        let b = x.to_bytes();
        core::array::from_fn(|i| u64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap()))
    };
    let neg_a_zisk = ziskos::zisklib::neg_point_ed25519(&a_zisk);
    tag_start!(double_scalar_mul_zisk);
    let r_zisk = ziskos::zisklib::double_scalar_mul_with_base_ed25519(
        black_box(&limbs(&k)),
        black_box(&neg_a_zisk),
        black_box(&limbs(&s)),
    );
    tag_end!(double_scalar_mul_zisk);
    assert_eq!(ziskos::zisklib::compress_ed25519(&r_zisk), r_bytes);

    // Differential check on rejected inputs: both verifiers must agree.
    let vk = VerifyingKey::from_bytes(&ED_PK).unwrap();
    let mut tampered_sig = ED_SIG_32;
    tampered_sig[0] ^= 1;
    let mut tampered_msg = m.clone();
    tampered_msg[0] ^= 1;
    for (msg, sig) in [(&m, &tampered_sig), (&tampered_msg, &ED_SIG_32)] {
        let dalek = vk.verify(msg, &Signature::from_bytes(sig)).is_ok();
        assert!(!dalek);
        assert_eq!(ziskos::zisklib::ed25519_verify(&ED_PK, msg, sig), dalek);
        assert_eq!(ziskos::zisklib::ed25519_verify_strict(&ED_PK, msg, sig), dalek);
    }

    tag_start!(mul_base_ct);
    let p = EdwardsPoint::mul_base(black_box(&k));
    tag_end!(mul_base_ct);
    black_box(p);
}
