//! Native tests of the Ed25519 module (precompiles and fcalls run through their host fallbacks).

use super::{
    add_points_ed25519, compress_ed25519,
    constants::{B, B_TABLE, IDENTITY, P, P_MINUS_ONE},
    decompress_ed25519, double_scalar_mul_with_base_ed25519, double_scalar_mul_with_base_ed25519_c,
    ed25519_verify, ed25519_verify_strict, is_on_curve_ed25519, is_small_order_ed25519,
    neg_point_ed25519,
    test_vectors::CASES,
};

#[test]
fn verify_matches_semantics() {
    for case in CASES {
        assert_eq!(
            ed25519_verify(&case.pk, case.msg, &case.sig),
            case.verify,
            "verify: {}",
            case.name
        );
        assert_eq!(
            ed25519_verify_strict(&case.pk, case.msg, &case.sig),
            case.verify_strict,
            "verify_strict: {}",
            case.name
        );
    }
}

#[test]
fn base_table_and_compression_roundtrip() {
    assert_eq!(B_TABLE[0], IDENTITY);
    assert_eq!(B_TABLE[1], B);
    for i in 1..16 {
        let next = add_points_ed25519(&B_TABLE[i - 1], &B);
        assert_eq!(next, B_TABLE[i], "B_TABLE[{i}]");
        assert_eq!(decompress_ed25519(&compress_ed25519(&B_TABLE[i])), Some(B_TABLE[i]));
        let neg = neg_point_ed25519(&B_TABLE[i]);
        assert_eq!(decompress_ed25519(&compress_ed25519(&neg)), Some(neg));
    }
}

#[test]
fn double_scalar_mul_matches_naive() {
    // [k]a + [s]B with small scalars against repeated additions
    let a = B_TABLE[3];
    for (k, s) in [(0u64, 0u64), (1, 0), (0, 1), (5, 7), (16, 255), (300, 17)] {
        let got = double_scalar_mul_with_base_ed25519(&[k, 0, 0, 0], &a, &[s, 0, 0, 0]);
        let mut expected = IDENTITY;
        for _ in 0..k {
            expected = add_points_ed25519(&expected, &a);
        }
        for _ in 0..s {
            expected = add_points_ed25519(&expected, &B);
        }
        assert_eq!(got, expected, "k={k} s={s}");
    }
}

#[test]
fn small_order_detection() {
    assert!(is_small_order_ed25519(&IDENTITY));
    assert!(!is_small_order_ed25519(&B));
    // (0, -1) has order 2
    let m = P_MINUS_ONE;
    assert!(is_small_order_ed25519(&[0, 0, 0, 0, m[0], m[1], m[2], m[3]]));
}

#[test]
fn on_curve_check() {
    assert!(is_on_curve_ed25519(&IDENTITY));
    assert!(is_on_curve_ed25519(&B));
    assert!(is_on_curve_ed25519(&neg_point_ed25519(&B_TABLE[5])));
    // (0, 2) is not on the curve
    assert!(!is_on_curve_ed25519(&[0, 0, 0, 0, 2, 0, 0, 0]));
    // (0, 1 + p) is the identity with a non-canonical y
    let y = [P[0] + 1, P[1], P[2], P[3]];
    assert!(!is_on_curve_ed25519(&[0, 0, 0, 0, y[0], y[1], y[2], y[3]]));
}

#[test]
fn double_scalar_mul_c_interface() {
    let to_bytes = |p: &[u64; 8]| -> [u8; 64] {
        let mut out = [0u8; 64];
        for (i, limb) in p.iter().enumerate() {
            out[8 * i..8 * i + 8].copy_from_slice(&limb.to_le_bytes());
        }
        out
    };
    let scalar_bytes = |k: &[u64; 4]| -> [u8; 32] {
        let mut out = [0u8; 32];
        for (i, limb) in k.iter().enumerate() {
            out[8 * i..8 * i + 8].copy_from_slice(&limb.to_le_bytes());
        }
        out
    };

    let a = neg_point_ed25519(&B_TABLE[7]);
    let k = [0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210, 0x1111_2222_3333_4444, u64::MAX];
    let s = [0xdead_beef, 0, 0x8000_0000_0000_0000, 0x0fff_ffff_ffff_ffff];
    let expected = double_scalar_mul_with_base_ed25519(&k, &a, &s);

    let mut result = [0u8; 64];
    let ok = unsafe {
        double_scalar_mul_with_base_ed25519_c(
            scalar_bytes(&k).as_ptr(),
            to_bytes(&a).as_ptr(),
            scalar_bytes(&s).as_ptr(),
            result.as_mut_ptr(),
        )
    };
    assert!(ok);
    assert_eq!(result, to_bytes(&expected));

    // Off-curve input is rejected and the output is left untouched
    let mut result = [0xAAu8; 64];
    let ok = unsafe {
        double_scalar_mul_with_base_ed25519_c(
            scalar_bytes(&k).as_ptr(),
            to_bytes(&[0, 0, 0, 0, 2, 0, 0, 0]).as_ptr(),
            scalar_bytes(&s).as_ptr(),
            result.as_mut_ptr(),
        )
    };
    assert!(!ok);
    assert_eq!(result, [0xAAu8; 64]);
}
