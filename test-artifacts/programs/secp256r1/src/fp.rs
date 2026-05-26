use ziskos::zisklib::{add_fp_secp256r1, mul_fp_secp256r1, square_fp_secp256r1};

use crate::constants::P_MINUS_ONE;

pub fn fp_tests() {
    // Addition: (p−1) + 1 ≡ 0 (mod p)
    let res = add_fp_secp256r1(&P_MINUS_ONE, &[1, 0, 0, 0]);
    assert_eq!(res, [0, 0, 0, 0]);

    // Addition: (p−1) + 2 ≡ 1 (mod p)
    let res = add_fp_secp256r1(&P_MINUS_ONE, &[2, 0, 0, 0]);
    assert_eq!(res, [1, 0, 0, 0]);

    // Multiplication: 0 · x = 0
    let x = [0x87d832983725d224, 0x798a9dbd05c98c74, 0x26624bb5fadfb817, 0x59622b41ba03b966];
    let res = mul_fp_secp256r1(&[0, 0, 0, 0], &x);
    assert_eq!(res, [0, 0, 0, 0]);

    // Multiplication: 1 · x = x
    let res = mul_fp_secp256r1(&[1, 0, 0, 0], &x);
    assert_eq!(res, x);

    // Multiplication: 2 · (p−1) = p − 2 (mod p)
    let p_minus_two: [u64; 4] =
        [P_MINUS_ONE[0] - 1, P_MINUS_ONE[1], P_MINUS_ONE[2], P_MINUS_ONE[3]];
    let res = mul_fp_secp256r1(&[2, 0, 0, 0], &P_MINUS_ONE);
    assert_eq!(res, p_minus_two);

    // Multiplication: (p−1)² ≡ 1 (mod p)
    let res = mul_fp_secp256r1(&P_MINUS_ONE, &P_MINUS_ONE);
    assert_eq!(res, [1, 0, 0, 0]);

    // Squaring: 2² = 4
    let res = square_fp_secp256r1(&[2, 0, 0, 0]);
    assert_eq!(res, [4, 0, 0, 0]);

    // Squaring: (p−1)² ≡ 1 (mod p)
    let res = square_fp_secp256r1(&P_MINUS_ONE);
    assert_eq!(res, [1, 0, 0, 0]);
}
