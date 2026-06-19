use ziskos::zisklib::{add_fp_secp256k1, mul_fp_secp256k1, sqrt_fp_secp256k1, square_fp_secp256k1};

/*
sage: p = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
sage: n = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
sage: F = GF(p)
sage: Fn = GF(n)
sage: E = EllipticCurve(F, [0,7])
sage: G = E(0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798, 0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8)
*/
pub fn fp_tests() {
    // Addition
    let a = [0x87d832983725d224, 0x798a9dbd05c98c74, 0x26624bb5fadfb817, 0x59622b41ba03b966];
    let b = [0x5f9f231bdd127ca1, 0xd61a40325833f333, 0x329f2b6e5826f1fb, 0x814e6375b67b17db];
    let res = add_fp_secp256k1(&a, &b);
    let res_exp = [0xe77755b414384ec5, 0x4fa4ddef5dfd7fa7, 0x590177245306aa13, 0xdab08eb7707ed141];
    assert_eq!(res, res_exp);

    // Multiplication
    let a = [0x87d832983725d224, 0x798a9dbd05c98c74, 0x26624bb5fadfb817, 0x59622b41ba03b966];
    let b = [0x5f9f231bdd127ca1, 0xd61a40325833f333, 0x329f2b6e5826f1fb, 0x814e6375b67b17db];
    let res = mul_fp_secp256k1(&a, &b);
    let res_exp = [0xaa2f9bcd686d24f6, 0x53ba237580c1ed1b, 0xae9ba1df41e261b8, 0xc85a601351bf65b9];
    assert_eq!(res, res_exp);

    // Squaring
    let a = [0x87d832983725d224, 0x798a9dbd05c98c74, 0x26624bb5fadfb817, 0x59622b41ba03b966];
    let res = square_fp_secp256k1(&a);
    let res_exp = [0x5dd3ad79e6737710, 0x7c6751b4ccd98b47, 0xfdc1575042b02a45, 0x691593f2fd2c7012];
    assert_eq!(res, res_exp);

    // Square Root
    let a = [0x87d832983725d226, 0x798a9dbd05c98c74, 0x26624bb5fadfb817, 0x59622b41ba03b966];
    let (res, is_quadratic) = sqrt_fp_secp256k1(&a, 0);
    let res_exp = [0xc75120f0e36700fe, 0x1ec8dac5f19fb98a, 0x276e4812fa862ed6, 0x438dbd7d330e4295];
    assert_eq!(res, res_exp);
    assert!(is_quadratic);

    let (res, is_quadratic) = sqrt_fp_secp256k1(&a, 1);
    let res_exp = [0x38aedf0e1c98fb31, 0xe137253a0e604675, 0xd891b7ed0579d129, 0xbc724282ccf1bd6a];
    assert_eq!(res, res_exp);
    assert!(is_quadratic);

    let a = [0x87d832983725d224, 0x798a9dbd05c98c74, 0x26624bb5fadfb817, 0x59622b41ba03b966];
    let (_, is_quadratic) = sqrt_fp_secp256k1(&a, 0);
    assert!(!is_quadratic);
}
