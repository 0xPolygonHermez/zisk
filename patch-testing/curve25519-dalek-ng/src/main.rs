//! curve25519-dalek-ng: Edwards point decompression routed to `zkvm_ed25519_decompress`, and
//! `EdwardsPoint::vartime_double_scalar_mul_basepoint` and `Scalar * EdwardsPoint` to
//! `zkvm_ed25519_double_scalar_mul_base`, exercised directly and through ed25519-consensus
//! (ZIP-215 verification).

#![no_main]
ziskos::entrypoint!(main);

use curve25519_dalek_ng::{
    constants::{ED25519_BASEPOINT_POINT, ED25519_BASEPOINT_TABLE, EIGHT_TORSION, RISTRETTO_BASEPOINT_POINT},
    edwards::{CompressedEdwardsY, EdwardsPoint},
    ristretto::RistrettoPoint,
    scalar::Scalar,
    traits::{IsIdentity, VartimeMultiscalarMul},
};
use ed25519_consensus::{Signature, SigningKey, VerificationKey};
use num_bigint::BigUint;
use patch_test_common::{hex, hex_array, Rng, Suite};

/// Independent decompression oracle: whether `enc` decodes to a curve point, and its canonical
/// encoding if so. `y` is taken mod p; `x² = (y² - 1) / (d·y² + 1)` must be a square (Euler's
/// criterion); a sign bit on `x = 0` is dropped. Matches curve25519-dalek's lenient rules.
fn decompress_oracle(enc: &[u8; 32]) -> Option<[u8; 32]> {
    let p = (BigUint::from(1u8) << 255u32) - 19u32;
    let inv = |a: &BigUint| a.modpow(&(&p - 2u32), &p);
    let d = ((&p - 121665u32) * inv(&BigUint::from(121666u32))) % &p;

    let mut y_bytes = *enc;
    let sign = y_bytes[31] >> 7;
    y_bytes[31] &= 0x7f;
    let y = BigUint::from_bytes_le(&y_bytes) % &p;
    let y2 = &y * &y % &p;
    let u = (&y2 + &p - 1u32) % &p;
    let v = (&d * &y2 + 1u32) % &p;
    let x2 = &u * inv(&v) % &p;
    let zero = BigUint::from(0u8);
    if x2 != zero && x2.modpow(&((&p - 1u32) >> 1), &p) != BigUint::from(1u8) {
        return None;
    }

    let mut out = [0u8; 32];
    let y_le = y.to_bytes_le();
    out[..y_le.len()].copy_from_slice(&y_le);
    if x2 != zero {
        out[31] |= sign << 7;
    }
    Some(out)
}

fn random_scalar(rng: &mut Rng) -> Scalar {
    let mut b = [0u8; 64];
    rng.fill(&mut b);
    Scalar::from_bytes_mod_order_wide(&b)
}

fn main() {
    let mut t = Suite::new("curve25519-dalek-ng");
    let mut rng = Rng::new(0xed25519);

    // ---- decompress ----

    // Basepoint (RFC 8032 encoding).
    let b_enc = hex_array::<32>("5866666666666666666666666666666666666666666666666666666666666666");
    t.eq("decompress(B)", CompressedEdwardsY(b_enc).decompress(), Some(ED25519_BASEPOINT_POINT));

    // Round trip of random points, against the oracle.
    for i in 0..16 {
        let p = &ED25519_BASEPOINT_TABLE * &random_scalar(&mut rng);
        let enc = p.compress();
        let q = enc.decompress();
        t.check(&format!("decompress(compress(P)) == P #{i}"), q == Some(p));
        t.eq(&format!("oracle(compress(P)) #{i}"), decompress_oracle(enc.as_bytes()), Some(enc.to_bytes()));
    }

    // Random strings (about half are not curve points) and the 19 non-canonical y = t + p
    // encodings, with both sign bits: validity and canonical re-encoding must match the oracle.
    let mut encodings = Vec::new();
    for _ in 0..48 {
        let mut e = [0u8; 32];
        rng.fill(&mut e);
        encodings.push(e);
    }
    for i in 0..19u8 {
        let mut e = [0xffu8; 32];
        e[0] = 0xed + i;
        e[31] = 0x7f;
        encodings.push(e);
        e[31] |= 0x80;
        encodings.push(e);
    }
    // (0, 1) and (0, -1) with the sign bit set on x = 0.
    encodings.push(hex_array("0100000000000000000000000000000000000000000000000000000000000080"));
    encodings.push(hex_array("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"));
    let (mut valid, mut invalid) = (0, 0);
    for e in &encodings {
        let got = CompressedEdwardsY(*e).decompress().map(|p| p.compress().to_bytes());
        let expected = decompress_oracle(e);
        if expected.is_some() { valid += 1 } else { invalid += 1 }
        t.eq(&format!("decompress({})", patch_test_common::to_hex(e)), got, expected);
    }
    t.check("decompress: both valid and invalid encodings covered", valid > 10 && invalid > 10);

    // ---- vartime_double_scalar_mul_basepoint and Scalar * EdwardsPoint ----
    // Reference: the Straus multiscalar multiplication, which the patch leaves on the upstream
    // code.

    let reference = |a: &Scalar, pa: &EdwardsPoint, b: &Scalar| {
        EdwardsPoint::vartime_multiscalar_mul(&[*a, *b], &[*pa, ED25519_BASEPOINT_POINT])
    };

    let mut points: Vec<EdwardsPoint> = Vec::new();
    for _ in 0..6 {
        // Projective (Z != 1) points from scalar multiplication...
        points.push(&ED25519_BASEPOINT_TABLE * &random_scalar(&mut rng));
    }
    for _ in 0..3 {
        // ...and affine ones from the accelerated decompression.
        let p = &ED25519_BASEPOINT_TABLE * &random_scalar(&mut rng);
        points.push(p.compress().decompress().unwrap());
    }
    // Mixed-order points: torsion components must survive the multiplication.
    points.push(points[0] + EIGHT_TORSION[1]);
    points.push(points[1] + EIGHT_TORSION[4]);
    points.extend_from_slice(&EIGHT_TORSION);

    for (i, pa) in points.iter().enumerate() {
        let (a, b) = (random_scalar(&mut rng), random_scalar(&mut rng));
        t.eq(
            &format!("[a]A + [b]B, point #{i}"),
            EdwardsPoint::vartime_double_scalar_mul_basepoint(&a, pa, &b).compress(),
            reference(&a, pa, &b).compress(),
        );
        t.eq(
            &format!("[a]A, point #{i}"),
            (&a * pa).compress(),
            EdwardsPoint::vartime_multiscalar_mul(&[a], &[*pa]).compress(),
        );
    }

    // is_torsion_free is [l]P == identity, through Scalar * EdwardsPoint.
    for (i, p) in points.iter().enumerate() {
        let expected = i < 9 || p.is_identity();
        t.eq(&format!("is_torsion_free, point #{i}"), p.is_torsion_free(), expected);
    }

    // Edge scalars.
    let pa = points[0];
    let minus_one = -Scalar::one();
    for (name, a, b) in [
        ("a = b = 0", Scalar::zero(), Scalar::zero()),
        ("a = 0", Scalar::zero(), random_scalar(&mut rng)),
        ("b = 0", random_scalar(&mut rng), Scalar::zero()),
        ("a = b = 1", Scalar::one(), Scalar::one()),
        ("a = b = l - 1", minus_one, minus_one),
        // Unreduced (< 2^255) scalars.
        ("a, b = 2^255 - 1", Scalar::from_bits([0xff; 32]), Scalar::from_bits([0xff; 32])),
    ] {
        t.eq(
            &format!("[a]A + [b]B, {name}"),
            EdwardsPoint::vartime_double_scalar_mul_basepoint(&a, &pa, &b).compress(),
            reference(&a, &pa, &b).compress(),
        );
        t.eq(
            &format!("[a]A, {name}"),
            (&a * &pa).compress(),
            EdwardsPoint::vartime_multiscalar_mul(&[a], &[pa]).compress(),
        );
    }
    t.check(
        "[l-1]B + [1]B = identity",
        EdwardsPoint::vartime_double_scalar_mul_basepoint(&minus_one, &ED25519_BASEPOINT_POINT, &Scalar::one())
            .is_identity(),
    );

    // The result is a usable point for further (upstream) arithmetic.
    let (a, b) = (random_scalar(&mut rng), random_scalar(&mut rng));
    let r = EdwardsPoint::vartime_double_scalar_mul_basepoint(&a, &pa, &b);
    t.eq("result + result == [2]result", (r + r).compress(), (&Scalar::from(2u8) * &r).compress());
    t.eq("[8]result == mul_by_cofactor", (&Scalar::from(8u8) * &r).compress(), r.mul_by_cofactor().compress());
    t.check("result is torsion-free", r.is_torsion_free());

    // Ristretto goes through the same Edwards function.
    for i in 0..3 {
        let pa = &RISTRETTO_BASEPOINT_POINT * &random_scalar(&mut rng);
        let (a, b) = (random_scalar(&mut rng), random_scalar(&mut rng));
        t.eq(
            &format!("ristretto [a]A + [b]B #{i}"),
            RistrettoPoint::vartime_double_scalar_mul_basepoint(&a, &pa, &b).compress(),
            RistrettoPoint::vartime_multiscalar_mul(&[a, b], &[pa, RISTRETTO_BASEPOINT_POINT]).compress(),
        );
    }

    // ---- ed25519-consensus ----

    // RFC 8032 section 7.1, tests 1-3 (pk derived from the seed, deterministic signatures).
    for (i, (seed, pk, sig, msg)) in [
        (
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
            "",
        ),
        (
            "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
            "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
            "72",
        ),
        (
            "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
            "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
            "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
            "af82",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let sk = SigningKey::from(hex_array::<32>(seed));
        let msg = hex(msg);
        let vk = VerificationKey::try_from(hex_array::<32>(pk)).expect("RFC 8032 key decodes");
        let sig = Signature::from(hex_array::<64>(sig));
        t.eq(&format!("rfc8032 #{} public key", i + 1), sk.verification_key().to_bytes(), vk.to_bytes());
        t.eq(&format!("rfc8032 #{} signature", i + 1), sk.sign(&msg).to_bytes(), sig.to_bytes());
        t.check(&format!("rfc8032 #{} verifies", i + 1), vk.verify(&sig, &msg).is_ok());

        let mut tampered = msg.clone();
        tampered.push(0);
        t.check(&format!("rfc8032 #{} rejects tampered msg", i + 1), vk.verify(&sig, &tampered).is_err());
        let mut bad = sig.to_bytes();
        bad[0] ^= 1;
        t.check(&format!("rfc8032 #{} rejects tampered R", i + 1), vk.verify(&Signature::from(bad), &msg).is_err());
    }

    // Sign/verify round trips on random keys and messages.
    for i in 0..8 {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        let sk = SigningKey::from(seed);
        let vk = sk.verification_key();
        let msg = rng.bytes(i * 37);
        let sig = sk.sign(&msg);
        t.check(&format!("random sign/verify #{i}"), vk.verify(&sig, &msg).is_ok());

        // s >= l is rejected: s + l still fits in 256 bits for any canonical s.
        let mut bytes = sig.to_bytes();
        let l = hex_array::<32>("edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010");
        let mut carry = 0u16;
        for j in 0..32 {
            let sum = bytes[32 + j] as u16 + l[j] as u16 + carry;
            bytes[32 + j] = sum as u8;
            carry = sum >> 8;
        }
        t.check(&format!("random s + l rejected #{i}"), vk.verify(&Signature::from(bytes), &msg).is_err());
    }

    // ZIP-215 small-order cases (the ed25519-consensus conformance set): A and R range over
    // the 8 canonical torsion encodings and the 6 non-canonical low-order encodings, s = 0.
    // [8][s]B = [8]R + [8][k]A then holds for every pair, so all 196 must verify; with s = 1
    // the left side is [8]B != 0, so none may.
    let mut small_order: Vec<[u8; 32]> = EIGHT_TORSION.iter().map(|p| p.compress().to_bytes()).collect();
    for e in [
        "0100000000000000000000000000000000000000000000000000000000000080",
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    ] {
        small_order.push(hex_array(e));
    }
    for (i, e) in small_order.iter().enumerate() {
        let p = CompressedEdwardsY(*e).decompress();
        t.check(&format!("small-order encoding #{i} decodes"), p.is_some_and(|p| p.is_small_order()));
    }
    let (mut ok_s0, mut ok_s1) = (0, 0);
    for a in &small_order {
        let vk = VerificationKey::try_from(*a).expect("small-order key decodes");
        for r in &small_order {
            let mut sig = [0u8; 64];
            sig[..32].copy_from_slice(r);
            ok_s0 += vk.verify(&Signature::from(sig), b"Zcash").is_ok() as u32;
            sig[32] = 1;
            ok_s1 += vk.verify(&Signature::from(sig), b"Zcash").is_ok() as u32;
        }
    }
    t.eq("zip215 small-order cases verifying with s = 0", ok_s0, 196);
    t.eq("zip215 small-order cases verifying with s = 1", ok_s1, 0);

    t.finish();
}
