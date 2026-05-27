use ziskos::zisklib::{
    double_scalar_mul_with_g_secp256r1, is_on_curve_secp256r1, scalar_mul_secp256r1,
};

use crate::constants::{G, G_NEG, IDENTITY, TWO_G};

pub fn curve_tests() {
    // Curve membership.
    assert_eq!(is_on_curve_secp256r1(&IDENTITY), false);
    assert_eq!(is_on_curve_secp256r1(&G), true);
    assert_eq!(is_on_curve_secp256r1(&G_NEG), true);
    assert_eq!(is_on_curve_secp256r1(&TWO_G), true);

    // Scalar multiplication.
    // 0 · G = 𝒪
    assert_eq!(scalar_mul_secp256r1(&[0, 0, 0, 0], &G), None);

    // 1 · G = G
    assert_eq!(scalar_mul_secp256r1(&[1, 0, 0, 0], &G), Some(G));

    // 2 · G — well-known NIST P-256 doubling vector.
    assert_eq!(scalar_mul_secp256r1(&[2, 0, 0, 0], &G), Some(TWO_G));

    // 3 · G via Strauss–Shamir is on the curve (we don't pin the value).
    let three_g = scalar_mul_secp256r1(&[3, 0, 0, 0], &G).expect("3·G must be defined");
    assert!(is_on_curve_secp256r1(&three_g));

    // Double scalar multiplication with G.
    // 0·G + 0·P = 𝒪
    assert_eq!(double_scalar_mul_with_g_secp256r1(&[0, 0, 0, 0], &[0, 0, 0, 0], &TWO_G), None);

    // 1·G + 0·P = G
    assert_eq!(double_scalar_mul_with_g_secp256r1(&[1, 0, 0, 0], &[0, 0, 0, 0], &TWO_G), Some(G));

    // 0·G + 1·P = P
    assert_eq!(
        double_scalar_mul_with_g_secp256r1(&[0, 0, 0, 0], &[1, 0, 0, 0], &TWO_G),
        Some(TWO_G)
    );

    // 1·G + 1·G via the (P=G, equal-scalars) branch:
    //   k1·G + k2·G = (k1+k2)·G   ⇒   1·G + 1·G = 2·G
    assert_eq!(double_scalar_mul_with_g_secp256r1(&[1, 0, 0, 0], &[1, 0, 0, 0], &G), Some(TWO_G));

    // 1·G + 1·(−G) = 𝒪
    assert_eq!(double_scalar_mul_with_g_secp256r1(&[1, 0, 0, 0], &[1, 0, 0, 0], &G_NEG), None);

    // 1·G + 1·(2G) = 3·G  — exercises the general Strauss–Shamir loop.
    let three_g_via_dsm = double_scalar_mul_with_g_secp256r1(&[1, 0, 0, 0], &[1, 0, 0, 0], &TWO_G)
        .expect("1·G + 1·2G must be defined");
    assert_eq!(three_g_via_dsm, three_g);
}
