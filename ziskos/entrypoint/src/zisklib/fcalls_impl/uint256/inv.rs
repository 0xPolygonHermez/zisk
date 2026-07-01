use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_integer::Integer;

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};

lazy_static! {
    pub(crate) static ref U256: BigUint = BigUint::parse_bytes(
        b"10000000000000000000000000000000000000000000000000000000000000000",
        16
    )
    .unwrap();
}

/// Compute `a^(-1) mod 2^256`, returning the result in 5 limbs: `[flag, inv[0..4]]`.
/// `flag` is 1 if the inverse exists (i.e. `a` is odd), 0 otherwise.
pub fn fcall_uint256_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a = &params[0..4].try_into().unwrap();

    // Perform the inversion
    let inv = uint256_inv(a);

    // Store the result
    match inv {
        Some(inv) => {
            results[0] = 1;
            results[1..5].copy_from_slice(&inv);
        }
        None => {
            results[0] = 0;
            results[1..5].copy_from_slice(&[0, 0, 0, 0]);
        }
    }

    5
}

pub fn uint256_inv(a: &[u64; 4]) -> Option<[u64; 4]> {
    let a_big = biguint_from_u64_digits(a);
    let inv = a_big.modinv(&U256);
    if let Some(inv) = inv {
        let inv_u64_digits = n_u64_digits_from_biguint::<4>(&inv);
        Some(inv_u64_digits)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_a_zero() {
        let a = [0, 0, 0, 0];
        let mut results = [0; 5];
        fcall_uint256_inv(&a, &mut results);
        let expected = [0, 0, 0, 0, 0];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_inv_a_one() {
        let a = [1, 0, 0, 0];
        let mut results = [0; 5];
        fcall_uint256_inv(&a, &mut results);
        let expected = [1, 1, 0, 0, 0];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_inv_a_even() {
        let a = [2, 3, 4, 5];
        let mut results = [0; 5];
        fcall_uint256_inv(&a, &mut results);
        let expected = [0, 0, 0, 0, 0];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_inv_a_odd1() {
        let a = [3, 0, 0, 0];
        let mut results = [0; 5];
        fcall_uint256_inv(&a, &mut results);
        let expected =
            [1, 0xaaaaaaaaaaaaaaab, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaaaaaaaaaa];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_inv_a_odd2() {
        let a = [0xee453cbb08caf011, 0x403f9ad46fdfbf18, 0x190bbcf54d8ad535, 0x9d4a5af226af865c];
        let mut results = [0; 5];
        fcall_uint256_inv(&a, &mut results);
        let expected =
            [1, 0x91f6316a1db400f1, 0xa62de0c72fbf1f2b, 0x8cc70b2dcf824747, 0x78bccb02bfaa76af];

        assert_eq!(results, expected);
    }
}
