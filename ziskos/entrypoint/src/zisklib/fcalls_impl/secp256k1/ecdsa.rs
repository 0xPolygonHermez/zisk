use num_traits::Zero;

use crate::zisklib::fcalls_impl::utils::{
    biguint_from_u64, biguint_from_u64_digits, n_u64_digits_from_biguint,
};

use super::constants::{G, IDENTITY, N, P};

pub fn fcall_secp256k1_ecdsa_verify(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let pk: &[u64; 8] = &params[0..8].try_into().unwrap();
    let z: &[u64; 4] = &params[8..12].try_into().unwrap();
    let r: &[u64; 4] = &params[12..16].try_into().unwrap();
    let s: &[u64; 4] = &params[16..20].try_into().unwrap();

    // Get the curve point P
    let p = secp256k1_ecdsa_verify(pk, z, r, s);

    // Store the result
    results[0..8].copy_from_slice(&p);

    8
}

pub fn secp256k1_ecdsa_verify(pk: &[u64; 8], z: &[u64; 4], r: &[u64; 4], s: &[u64; 4]) -> [u64; 8] {
    // Given the public key pk and the signature (r, s) over the message hash z:
    // 1. Computes s_inv = s⁻¹ mod n
    // 2. Computes u1 = z·s_inv mod n
    // 3. Computes u2 = r·s_inv mod n
    // 4. Computes and returns the curve point p = u1·G + u2·PK
    let s_inv = secp256k1_fn_inv(s);
    let u1 = secp256k1_fn_mul(z, &s_inv);
    let u2 = secp256k1_fn_mul(r, &s_inv);
    secp256k1_curve_dbl_scalar_mul(&u1, &G, &u2, pk)
}

fn secp256k1_fn_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let product = (a_big * b_big) % &*N;
    n_u64_digits_from_biguint(&product)
}

fn secp256k1_fn_inv(a: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let inv = a_big.modinv(&N);
    match inv {
        Some(inverse) => n_u64_digits_from_biguint(&inverse),
        None => panic!("Inverse does not exist"),
    }
}

fn secp256k1_fp_add(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let sum = (a_big + b_big) % &*P;
    n_u64_digits_from_biguint(&sum)
}

fn secp256k1_fp_sub(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let diff = if a_big >= b_big { a_big - b_big } else { (a_big + &*P) - b_big };
    n_u64_digits_from_biguint(&diff)
}

fn secp256k1_fp_scalar_mul(a: &[u64; 4], scalar: u64) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let scalar_big = biguint_from_u64(scalar);
    let product = (a_big * scalar_big) % &*P;
    n_u64_digits_from_biguint(&product)
}

fn secp256k1_fp_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let product = (a_big * b_big) % &*P;
    n_u64_digits_from_biguint(&product)
}

fn secp256k1_fp_square(a: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let square = (a_big.clone() * a_big) % &*P;
    n_u64_digits_from_biguint(&square)
}

fn secp256k1_fp_inv(a: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let inv = a_big.modinv(&P);
    match inv {
        Some(inverse) => n_u64_digits_from_biguint(&inverse),
        None => panic!("Inverse does not exist"),
    }
}

fn secp256k1_curve_add(p: &[u64; 8], q: &[u64; 8]) -> [u64; 8] {
    let x1: &[u64; 4] = &p[0..4].try_into().unwrap();
    let y1: &[u64; 4] = &p[4..8].try_into().unwrap();
    let x2: &[u64; 4] = &q[0..4].try_into().unwrap();
    let y2: &[u64; 4] = &q[4..8].try_into().unwrap();

    if x1 == x2 {
        if y1 == y2 {
            return secp256k1_curve_dbl(p);
        } else {
            return IDENTITY;
        }
    }

    if p == &IDENTITY {
        return *q;
    } else if q == &IDENTITY {
        return *p;
    }

    let lambda = {
        let y2_minus_y1 = secp256k1_fp_sub(y2, y1);
        let x2_minus_x1 = secp256k1_fp_sub(x2, x1);
        let x2_minus_x1_inv = secp256k1_fp_inv(&x2_minus_x1);
        secp256k1_fp_mul(&y2_minus_y1, &x2_minus_x1_inv)
    };

    let x3 = {
        let lambda_sq = secp256k1_fp_square(&lambda);
        let x1_plus_x2 = secp256k1_fp_add(x1, x2);
        secp256k1_fp_sub(&lambda_sq, &x1_plus_x2)
    };

    let y3 = {
        let lambda_x1_minus_x3 = {
            let x1_minus_x3 = secp256k1_fp_sub(x1, &x3);
            secp256k1_fp_mul(&lambda, &x1_minus_x3)
        };
        secp256k1_fp_sub(&lambda_x1_minus_x3, y1)
    };

    let mut result = [0u64; 8];
    result[0..4].copy_from_slice(&x3);
    result[4..8].copy_from_slice(&y3);
    result
}

fn secp256k1_curve_dbl(p: &[u64; 8]) -> [u64; 8] {
    if p == &IDENTITY {
        return *p;
    }

    let x: &[u64; 4] = &p[0..4].try_into().unwrap();
    let y: &[u64; 4] = &p[4..8].try_into().unwrap();

    let lambda = {
        let three_x1_sq = {
            let x1_sq = secp256k1_fp_square(x);
            secp256k1_fp_scalar_mul(&x1_sq, 3)
        };

        let two_y1 = secp256k1_fp_scalar_mul(y, 2);
        let two_y1_inv = secp256k1_fp_inv(&two_y1);

        secp256k1_fp_mul(&three_x1_sq, &two_y1_inv)
    };

    let x3 = {
        let lambda_sq = secp256k1_fp_square(&lambda);
        let two_x1 = secp256k1_fp_scalar_mul(x, 2);
        secp256k1_fp_sub(&lambda_sq, &two_x1)
    };

    let y3 = {
        let lambda_x1_minus_x3 = {
            let x1_minus_x3 = secp256k1_fp_sub(x, &x3);
            secp256k1_fp_mul(&lambda, &x1_minus_x3)
        };
        secp256k1_fp_sub(&lambda_x1_minus_x3, y)
    };

    let mut result = [0u64; 8];
    result[0..4].copy_from_slice(&x3);
    result[4..8].copy_from_slice(&y3);
    result
}

fn secp256k1_curve_dbl_scalar_mul(
    k1: &[u64; 4],
    p1: &[u64; 8],
    k2: &[u64; 4],
    p2: &[u64; 8],
) -> [u64; 8] {
    let mut r = IDENTITY;
    for i in (0..256).rev() {
        r = secp256k1_curve_dbl(&r);

        let k1_bit = (k1[i / 64] >> (i % 64)) & 1;
        let k2_bit = (k2[i / 64] >> (i % 64)) & 1;

        if k1_bit == 1 {
            r = secp256k1_curve_add(&r, p1);
        }
        if k2_bit == 1 {
            r = secp256k1_curve_add(&r, p2);
        }
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbl_scalar_mul() {
        // 0 * IDENTITY + 0 * IDENTITY = IDENTITY
        let k1 = [0u64; 4];
        let p1 = IDENTITY;
        let k2 = [0u64; 4];
        let p2 = IDENTITY;

        let result = secp256k1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        assert_eq!(result, IDENTITY);

        // 1 * G + 0 * IDENTITY = G
        let k1 = [1u64, 0, 0, 0];
        let p1 = G;
        let k2 = [0u64; 4];
        let p2 = IDENTITY;

        let result = secp256k1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        assert_eq!(result, G);

        // 0 * IDENTITY + 1 * G = G
        let k1 = [0u64; 4];
        let p1 = IDENTITY;
        let k2 = [1u64, 0, 0, 0];
        let p2 = G;

        let result = secp256k1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        assert_eq!(result, G);

        // 2 * G + 3 * G = 5 * G
        let k1 = [2u64, 0, 0, 0];
        let p1 = G;
        let k2 = [3u64, 0, 0, 0];
        let p2 = G;

        let result = secp256k1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        let expected = [
            0xcba8d569b240efe4,
            0xe88b84bddc619ab7,
            0x55b4a7250a5c5128,
            0x2f8bde4d1a072093,
            0xdca87d3aa6ac62d6,
            0xf788271bab0d6840,
            0xd4dba9dda6c9c426,
            0xd8ac222636e5e3d6,
        ];
        assert_eq!(result, expected);

        // Random test
        let k1 = [0x761923728d37303, 0x1f0e6f2fa8a32ab5, 0x7bb7458c6ea47f08, 0xe2cf4fd21aef19e1];
        let p1 = [
            0xd77a8f3f445d2c43,
            0xd8404b226e191e33,
            0x3f542469b3a1f4ce,
            0x73613de6799853d9,
            0x9722df4889803b47,
            0x9055e100179fe79a,
            0xdf46f38d013fda72,
            0xd769a27efc36598c,
        ];
        let k2 = [0xe9c44fa1510380c0, 0x16d1daea9be6a28, 0x2a4bb6bbdc0a031e, 0xefda864ae6c22f24];
        let p2 = [
            0x77fb10949fdba7d6,
            0x84e5d96e491b9daf,
            0x66c77ea552e760cd,
            0x434feb1463e34ff8,
            0x5258fc8877bdff59,
            0x25586ed50053a57f,
            0x55858e1de54a18ac,
            0x3393bec7dd4067f7,
        ];

        let result = secp256k1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        let expected = [
            0xb0531ccb6c1c9b1,
            0xc7c48529c9569495,
            0x18edf1edb9351c8d,
            0x572d78c95d7f964,
            0x9d41caf8f65f3690,
            0x21eea422b3a37e0a,
            0x1c10371d5a68938c,
            0xcc37bbabaf4204de,
        ];
        assert_eq!(result, expected);
    }
}
