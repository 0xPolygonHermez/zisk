use lazy_static::lazy_static;
use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::zisklib::fcalls_impl::utils::{bigint_from_u64_digits, n_u64_digits_from_biguint};

use super::constants::N;

lazy_static! {
    /// Short-basis vector v1 = (A1, B1) with A1 + B1·λ ≡ 0 (mod n). B1 is negative.
    static ref A1: BigInt = BigInt::parse_bytes(
        b"3086D221A7D46BCDE86C90E49284EB15", 16
    ).unwrap();
    static ref MINUS_B1: BigInt = BigInt::parse_bytes(
        b"E4437ED6010E88286F547FA90ABFE4C3", 16
    ).unwrap();

    /// Short-basis vector v2 = (A2, B2) with A2 + B2·λ ≡ 0 (mod n). Note B2 == A1.
    static ref A2: BigInt = BigInt::parse_bytes(
        b"114CA50F7A8E2F3F657C1108D9D44CFD8", 16
    ).unwrap();
    static ref B2: BigInt = A1.clone();

    static ref N_INT: BigInt = BigInt::from(N.clone());
    static ref TWO_128: BigInt = BigInt::one() << 128;
}

/// Given a scalar `k ∈ [0, n)`, splits it into `(k1, k2)` with `|k1|, |k2| < 2^128` such that
/// `k ≡ k1 + k2·λ (mod n)`, where `λ` is the cube root of unity that secp256k1's endomorphism
/// acts as.
///
/// Returns `(k1_abs, k2_abs, sigma1, sigma2)` where `sigma_i ∈ {0, 1}`:
/// - `0` means positive,
/// - `1` means negative.
pub fn fcall_secp256k1_glv_decompose(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input scalar k
    let k: &[u64; 4] = params[0..4].try_into().unwrap();

    // c1 ≈ round(B2·k/n), c2 ≈ round(-B1·k/n)
    let k_int = bigint_from_u64_digits(k);
    let c1 = round_div(&(&*B2 * &k_int), &N_INT);
    let c2 = round_div(&(&*MINUS_B1 * &k_int), &N_INT);

    // k1 = k - c1·A1 - c2·A2
    // k2 = -c1·B1 - c2·B2 = c1·(-B1) - c2·B2
    let k1 = &k_int - &c1 * &*A1 - &c2 * &*A2;
    let k2 = &c1 * &*MINUS_B1 - &c2 * &*B2;

    debug_assert!(k1.abs() < *TWO_128, "GLV decomposition: |k1| >= 2^128");
    debug_assert!(k2.abs() < *TWO_128, "GLV decomposition: |k2| >= 2^128");

    // Sanity check: k1 + k2·λ ≡ k (mod n)
    #[cfg(debug_assertions)]
    {
        use num_bigint::ToBigInt;
        let lambda_int = BigInt::parse_bytes(
            b"5363AD4CC05C30E0A5261C028812645A122E22EA20816678DF02967C1B23BD72",
            16,
        )
        .unwrap();
        let lhs = (&k1 + &k2 * &lambda_int).mod_floor(&N_INT);
        let rhs = k_int.mod_floor(&N_INT);
        debug_assert_eq!(lhs, rhs, "GLV decomposition relation failed");
        let _ = lambda_int.to_bigint();
    }

    let (sigma1, k1_abs) = bigint_to_unsigned(k1);
    let (sigma2, k2_abs) = bigint_to_unsigned(k2);

    let k1_limbs: [u64; 4] = n_u64_digits_from_biguint(&k1_abs);
    let k2_limbs: [u64; 4] = n_u64_digits_from_biguint(&k2_abs);
    results[0..4].copy_from_slice(&k1_limbs);
    results[4..8].copy_from_slice(&k2_limbs);
    results[8] = sigma1;
    results[9] = sigma2;
    10
}

/// Computes `round(num / den)` for non-negative `den` (`num` may be signed).
fn round_div(num: &BigInt, den: &BigInt) -> BigInt {
    debug_assert!(!den.is_negative() && !den.is_zero());

    // round(num/den) = floor((2·num + den) / (2·den)) for positive num,
    //                = -floor((2·|num| + den) / (2·den)) for negative num.
    let two_den = den << 1;
    if num.is_negative() {
        let q: BigInt = (-num * 2 + den) / &two_den;
        -q
    } else {
        (num * 2 + den) / &two_den
    }
}

fn bigint_to_unsigned(x: BigInt) -> (u64, BigUint) {
    let (sign, mag) = x.into_parts();
    match sign {
        Sign::Minus => (1, mag),
        Sign::NoSign | Sign::Plus => (0, mag),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zisklib::fcalls_impl::utils::biguint_from_u64_digits;

    fn lambda_int() -> BigInt {
        BigInt::parse_bytes(b"5363AD4CC05C30E0A5261C028812645A122E22EA20816678DF02967C1B23BD72", 16)
            .unwrap()
    }

    /// Runs the decomposition and asserts:
    /// 1. magnitudes fit in 128 bits,
    /// 2. sigma bits are 0 or 1,
    /// 3. (-1)^σ₁·k₁ + (-1)^σ₂·k₂·λ ≡ k (mod n).
    fn check_decompose(k_limbs: [u64; 4]) {
        let params = k_limbs;
        let mut results = [0u64; 10];
        let n_written = fcall_secp256k1_glv_decompose(&params, &mut results);
        assert_eq!(n_written, 10);

        let k1 = biguint_from_u64_digits(&results[0..4]);
        let k2 = biguint_from_u64_digits(&results[4..8]);
        let sigma1 = results[8];
        let sigma2 = results[9];

        assert!(k1 < BigUint::one() << 128, "|k1| >= 2^128");
        assert!(k2 < BigUint::one() << 128, "|k2| >= 2^128");
        assert!(sigma1 <= 1, "sigma1 not a bit");
        assert!(sigma2 <= 1, "sigma2 not a bit");

        let k1_signed = if sigma1 == 0 { BigInt::from(k1) } else { -BigInt::from(k1) };
        let k2_signed = if sigma2 == 0 { BigInt::from(k2) } else { -BigInt::from(k2) };

        let lhs = (&k1_signed + &k2_signed * &lambda_int()).mod_floor(&N_INT);
        let rhs = BigInt::from(biguint_from_u64_digits(&k_limbs)).mod_floor(&N_INT);
        assert_eq!(lhs, rhs, "k1 + k2·λ != k (mod n)");
    }

    #[test]
    fn glv_basis_relation() {
        // Sanity check on the hard-coded constants:
        // A1 + B1·λ ≡ 0 (mod n) and A2 + B2·λ ≡ 0 (mod n).
        let lambda = lambda_int();
        let minus_b1 = MINUS_B1.clone();
        let b1 = -minus_b1; // B1 is negative
        let rel1 = (&*A1 + &b1 * &lambda).mod_floor(&N_INT);
        assert_eq!(rel1, BigInt::zero(), "A1 + B1·λ != 0 mod n");

        let rel2 = (&*A2 + &*B2 * &lambda).mod_floor(&N_INT);
        assert_eq!(rel2, BigInt::zero(), "A2 + B2·λ != 0 mod n");
    }

    #[test]
    fn glv_decompose_zero() {
        // k = 0 ⇒ (k1, k2) = (0, 0)
        let mut results = [0u64; 10];
        fcall_secp256k1_glv_decompose(&[0, 0, 0, 0], &mut results);
        assert_eq!(&results[0..8], &[0u64; 8]);
        check_decompose([0, 0, 0, 0]);
    }

    #[test]
    fn glv_decompose_lambda_itself() {
        // k = λ ⇒ trivially (k1, k2) = (0, 1). A nice sanity test for the relation.
        let lambda_limbs: [u64; 4] =
            [0xDF02967C1B23BD72, 0x122E22EA20816678, 0xA5261C028812645A, 0x5363AD4CC05C30E0];
        check_decompose(lambda_limbs);
    }

    #[test]
    fn glv_decompose_small() {
        // k = 1, 2, 7
        check_decompose([1, 0, 0, 0]);
        check_decompose([2, 0, 0, 0]);
        check_decompose([7, 0, 0, 0]);
    }

    #[test]
    fn glv_decompose_near_n() {
        // n - 1 (the largest valid scalar)
        let n_minus_one: [u64; 4] =
            [0xBFD25E8CD0364140, 0xBAAEDCE6AF48A03B, 0xFFFFFFFFFFFFFFFE, 0xFFFFFFFFFFFFFFFF];
        check_decompose(n_minus_one);

        // n/2 area
        check_decompose([
            0x5FE92F46681B20A0,
            0x5D576E7357A45051,
            0x7FFFFFFFFFFFFFFF,
            0x7FFFFFFFFFFFFFFF,
        ]);
    }

    #[test]
    fn glv_decompose_random_vectors() {
        let samples: [[u64; 4]; 4] = [
            [0xf447e442a44c829e, 0xe979220cfe9824d3, 0x673913d78b5bdbfe, 0xd961172287f69999],
            [0x9249014d999485b7, 0xdfd89459c31678cb, 0x4436e3fc08fe4970, 0x849f0f75e5ce061b],
            [0xbfa9bf56ca443f1e, 0x9b50eab82f329ea5, 0x758838002e2f0ec7, 0x1fa7537a493bfc54],
            [0x0123456789ABCDEF, 0xFEDCBA9876543210, 0xDEADBEEFCAFEBABE, 0x0F1E2D3C4B5A6978],
        ];
        for k in samples.iter() {
            check_decompose(*k);
        }
    }
}
