use ziskos::zisklib::{
    add_fn_secp256r1, inv_fn_secp256r1, mul_fn_secp256r1, neg_fn_secp256r1, reduce_fn_secp256r1,
    sub_fn_secp256r1,
};

use crate::constants::{N, N_MINUS_ONE};

pub fn scalar_tests() {
    // Reduction: n ≡ 0 (mod n)
    let res = reduce_fn_secp256r1(&N);
    assert_eq!(res, [0, 0, 0, 0]);

    // Reduction: (n−1) ≡ n−1 (mod n) — fast path (x < n returns x unchanged).
    let res = reduce_fn_secp256r1(&N_MINUS_ONE);
    assert_eq!(res, N_MINUS_ONE);

    // Reduction: (n + 1) ≡ 1 (mod n)
    let n_plus_one: [u64; 4] = [N[0] + 1, N[1], N[2], N[3]];
    let res = reduce_fn_secp256r1(&n_plus_one);
    assert_eq!(res, [1, 0, 0, 0]);

    // Addition: (n−1) + 1 ≡ 0
    let res = add_fn_secp256r1(&N_MINUS_ONE, &[1, 0, 0, 0]);
    assert_eq!(res, [0, 0, 0, 0]);

    // Addition: (n−1) + 2 ≡ 1
    let res = add_fn_secp256r1(&N_MINUS_ONE, &[2, 0, 0, 0]);
    assert_eq!(res, [1, 0, 0, 0]);

    // Negation: −0 ≡ 0
    let res = neg_fn_secp256r1(&[0, 0, 0, 0]);
    assert_eq!(res, [0, 0, 0, 0]);

    // Negation: −1 ≡ n−1
    let res = neg_fn_secp256r1(&[1, 0, 0, 0]);
    assert_eq!(res, N_MINUS_ONE);

    // Subtraction: 0 − 1 ≡ n−1
    let res = sub_fn_secp256r1(&[0, 0, 0, 0], &[1, 0, 0, 0]);
    assert_eq!(res, N_MINUS_ONE);

    // Subtraction: (n−1) − (n−1) ≡ 0
    let res = sub_fn_secp256r1(&N_MINUS_ONE, &N_MINUS_ONE);
    assert_eq!(res, [0, 0, 0, 0]);

    // Multiplication: 0 · x = 0
    let x = [0x334d5469d32c3b5b, 0x2b7465755356f643, 0x60e777bde950c3b6, 0x3db52491030af31e];
    let res = mul_fn_secp256r1(&[0, 0, 0, 0], &x);
    assert_eq!(res, [0, 0, 0, 0]);

    // Multiplication: 1 · x = x
    let res = mul_fn_secp256r1(&[1, 0, 0, 0], &x);
    assert_eq!(res, x);

    // Multiplication: 2 · (n−1) ≡ n−2 (mod n)
    let n_minus_two: [u64; 4] =
        [N_MINUS_ONE[0] - 1, N_MINUS_ONE[1], N_MINUS_ONE[2], N_MINUS_ONE[3]];
    let res = mul_fn_secp256r1(&[2, 0, 0, 0], &N_MINUS_ONE);
    assert_eq!(res, n_minus_two);

    // Inverse: 1⁻¹ ≡ 1 (mod n)
    let res = inv_fn_secp256r1(&[1, 0, 0, 0]);
    assert_eq!(res, [1, 0, 0, 0]);

    // Inverse: 2⁻¹ ≡ (n+1)/2 (mod n). Verify via 2 · inv ≡ 1.
    let inv2 = inv_fn_secp256r1(&[2, 0, 0, 0]);
    let two_inv2 = mul_fn_secp256r1(&[2, 0, 0, 0], &inv2);
    assert_eq!(two_inv2, [1, 0, 0, 0]);

    // Inverse round-trip: x · x⁻¹ ≡ 1 for a random-ish non-zero x.
    let x_inv = inv_fn_secp256r1(&x);
    let prod = mul_fn_secp256r1(&x, &x_inv);
    assert_eq!(prod, [1, 0, 0, 0]);
}
