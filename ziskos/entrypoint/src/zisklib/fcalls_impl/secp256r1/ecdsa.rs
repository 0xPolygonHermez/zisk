use num_traits::Zero;

use crate::zisklib::fcalls_impl::utils::{
    biguint_from_u64, biguint_from_u64_digits, n_u64_digits_from_biguint,
};

use super::constants::{E_A, G, IDENTITY, N, P};

pub fn fcall_secp256r1_ecdsa_verify(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let pk: &[u64; 8] = &params[0..8].try_into().unwrap();
    let z: &[u64; 4] = &params[8..12].try_into().unwrap();
    let r: &[u64; 4] = &params[12..16].try_into().unwrap();
    let s: &[u64; 4] = &params[16..20].try_into().unwrap();

    // Get the curve point P
    let p = secp256r1_ecdsa_verify(pk, z, r, s);

    // Store the result
    results[0..8].copy_from_slice(&p);

    8
}

fn secp256r1_ecdsa_verify(pk: &[u64; 8], z: &[u64; 4], r: &[u64; 4], s: &[u64; 4]) -> [u64; 8] {
    // Given the public key pk and the signature (r, s) over the message hash z:
    // 1. Computes s_inv = s⁻¹ mod n
    // 2. Computes u1 = z·s_inv mod n
    // 3. Computes u2 = r·s_inv mod n
    // 4. Computes and returns the curve point p = u1·G + u2·PK
    let s_inv = secp256r1_fn_inv(s);
    let u1 = secp256r1_fn_mul(z, &s_inv);
    let u2 = secp256r1_fn_mul(r, &s_inv);
    secp256r1_curve_dbl_scalar_mul(&u1, &G, &u2, pk)
}

fn secp256r1_fn_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let product = (a_big * b_big) % &*N;
    n_u64_digits_from_biguint(&product)
}

fn secp256r1_fn_inv(a: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let inv = a_big.modinv(&N);
    match inv {
        Some(inverse) => n_u64_digits_from_biguint(&inverse),
        None => panic!("Inverse does not exist"),
    }
}

fn secp256r1_fp_add(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let sum = (a_big + b_big) % &*P;
    n_u64_digits_from_biguint(&sum)
}

fn secp256r1_fp_sub(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let diff = if a_big >= b_big { a_big - b_big } else { (a_big + &*P) - b_big };
    n_u64_digits_from_biguint(&diff)
}

fn secp256r1_fp_scalar_mul(a: &[u64; 4], scalar: u64) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let scalar_big = biguint_from_u64(scalar);
    let product = (a_big * scalar_big) % &*P;
    n_u64_digits_from_biguint(&product)
}

fn secp256r1_fp_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let product = (a_big * b_big) % &*P;
    n_u64_digits_from_biguint(&product)
}

fn secp256r1_fp_square(a: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let square = (a_big.clone() * a_big) % &*P;
    n_u64_digits_from_biguint(&square)
}

fn secp256r1_fp_inv(a: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let inv = a_big.modinv(&P);
    match inv {
        Some(inverse) => n_u64_digits_from_biguint(&inverse),
        None => panic!("Inverse does not exist"),
    }
}

fn secp256r1_curve_add(p: &[u64; 8], q: &[u64; 8]) -> [u64; 8] {
    let x1: &[u64; 4] = &p[0..4].try_into().unwrap();
    let y1: &[u64; 4] = &p[4..8].try_into().unwrap();
    let x2: &[u64; 4] = &q[0..4].try_into().unwrap();
    let y2: &[u64; 4] = &q[4..8].try_into().unwrap();

    if x1 == x2 {
        if y1 == y2 {
            return secp256r1_curve_dbl(p);
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
        let y2_minus_y1 = secp256r1_fp_sub(y2, y1);
        let x2_minus_x1 = secp256r1_fp_sub(x2, x1);
        let x2_minus_x1_inv = secp256r1_fp_inv(&x2_minus_x1);
        secp256r1_fp_mul(&y2_minus_y1, &x2_minus_x1_inv)
    };

    let x3 = {
        let lambda_sq = secp256r1_fp_square(&lambda);
        let x1_plus_x2 = secp256r1_fp_add(x1, x2);
        secp256r1_fp_sub(&lambda_sq, &x1_plus_x2)
    };

    let y3 = {
        let lambda_x1_minus_x3 = {
            let x1_minus_x3 = secp256r1_fp_sub(x1, &x3);
            secp256r1_fp_mul(&lambda, &x1_minus_x3)
        };
        secp256r1_fp_sub(&lambda_x1_minus_x3, y1)
    };

    let mut result = [0u64; 8];
    result[0..4].copy_from_slice(&x3);
    result[4..8].copy_from_slice(&y3);
    result
}

fn secp256r1_curve_dbl(p: &[u64; 8]) -> [u64; 8] {
    if p == &IDENTITY {
        return *p;
    }

    let x: &[u64; 4] = &p[0..4].try_into().unwrap();
    let y: &[u64; 4] = &p[4..8].try_into().unwrap();

    let lambda = {
        let three_x1_sq = {
            let x1_sq = secp256r1_fp_square(x);
            secp256r1_fp_scalar_mul(&x1_sq, 3)
        };
        let num = secp256r1_fp_add(&three_x1_sq, &E_A);

        let two_y1 = secp256r1_fp_scalar_mul(y, 2);
        let den = secp256r1_fp_inv(&two_y1);

        secp256r1_fp_mul(&num, &den)
    };

    let x3 = {
        let lambda_sq = secp256r1_fp_square(&lambda);
        let two_x1 = secp256r1_fp_scalar_mul(x, 2);
        secp256r1_fp_sub(&lambda_sq, &two_x1)
    };

    let y3 = {
        let lambda_x1_minus_x3 = {
            let x1_minus_x3 = secp256r1_fp_sub(x, &x3);
            secp256r1_fp_mul(&lambda, &x1_minus_x3)
        };
        secp256r1_fp_sub(&lambda_x1_minus_x3, y)
    };

    let mut result = [0u64; 8];
    result[0..4].copy_from_slice(&x3);
    result[4..8].copy_from_slice(&y3);
    result
}

fn secp256r1_curve_dbl_scalar_mul(
    k1: &[u64; 4],
    p1: &[u64; 8],
    k2: &[u64; 4],
    p2: &[u64; 8],
) -> [u64; 8] {
    let mut r = IDENTITY;
    for i in (0..256).rev() {
        r = secp256r1_curve_dbl(&r);

        let k1_bit = (k1[i / 64] >> (i % 64)) & 1;
        let k2_bit = (k2[i / 64] >> (i % 64)) & 1;

        if k1_bit == 1 {
            r = secp256r1_curve_add(&r, p1);
        }
        if k2_bit == 1 {
            r = secp256r1_curve_add(&r, p2);
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

        let result = secp256r1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        assert_eq!(result, IDENTITY);

        // 1 * G + 0 * IDENTITY = G
        let k1 = [1u64, 0, 0, 0];
        let p1 = G;
        let k2 = [0u64; 4];
        let p2 = IDENTITY;

        let result = secp256r1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        assert_eq!(result, G);

        // 0 * IDENTITY + 1 * G = G
        let k1 = [0u64; 4];
        let p1 = IDENTITY;
        let k2 = [1u64, 0, 0, 0];
        let p2 = G;

        let result = secp256r1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        assert_eq!(result, G);

        // 2 * G + 3 * G = 5 * G
        let k1 = [2u64, 0, 0, 0];
        let p1 = G;
        let k2 = [3u64, 0, 0, 0];
        let p2 = G;

        let result = secp256r1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        let expected = [
            0x21554a0dc3d033ed,
            0xef8c82fd1f5be524,
            0xd784c85608668fdf,
            0x51590b7a515140d2,
            0xd1d0bb44fda16da4,
            0xd012f00d4d80888,
            0x8ae1bf36bf8a7926,
            0xe0c17da8904a727d,
        ];
        assert_eq!(result, expected);

        // Random test
        let k1 = [0xc4bed2f1f47f9a54, 0x9cd109ce498a9b95, 0xd9d5232066758816, 0xf3b0020b50fafcfe];
        let p1 = [
            0x3c86442bafe51c41,
            0xa709f983d1ad2017,
            0x503d3c4c7699e29f,
            0x51f730041a088667,
            0xb4c365119c4d3bfc,
            0x41f620cca7b9001f,
            0xeb5025341faef867,
            0xf55cbe6ac6ff94ce,
        ];
        let k2 = [0xb652a5b177426eaa, 0xe44bcf080ef8aaf7, 0x3966826b0d4eb5f5, 0xe33606d47d23f70a];
        let p2 = [
            0x8ba8cddeb162e15b,
            0xb33b65b9a6c8945c,
            0x7480c2cff5cea8e0,
            0x3393c7d67a51330d,
            0xf1d29bdb9ed24e90,
            0x5da65af891bf0b50,
            0x99cc7a2be908e44e,
            0x8de4594f14dc559d,
        ];

        let result = secp256r1_curve_dbl_scalar_mul(&k1, &p1, &k2, &p2);
        let expected = [
            0xa57ec301274eaa5c,
            0x7f2d4f49c426a01b,
            0x910612a4889b8c13,
            0x4436050010e76a1e,
            0x2cd45c4320036102,
            0xc2d5e53a2316da0a,
            0x76355a97180de3fe,
            0xd15d039ba7950631,
        ];
        assert_eq!(result, expected);
    }
}
