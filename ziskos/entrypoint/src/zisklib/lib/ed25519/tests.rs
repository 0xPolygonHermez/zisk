//! Native tests of the Ed25519 module (precompiles and fcalls run through their host fallbacks).

use super::{
    add_points_ed25519, compress_ed25519,
    constants::{B, B_TABLE, IDENTITY, P_MINUS_ONE},
    decompress_ed25519, double_scalar_mul_with_base_ed25519, ed25519_verify, ed25519_verify_strict,
    is_small_order_ed25519, neg_point_ed25519,
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
