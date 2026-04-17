use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_integer::Integer;

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};

/// Compute `a^(-1) mod modulus`, returning 5 limbs: `[flag, inv[0..4]]`.
/// `flag` is 1 if the inverse exists, 0 otherwise.
pub fn fcall_uint256_inv_mod(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a = &params[0..4].try_into().unwrap();
    let modulus = &params[4..8].try_into().unwrap();

    // Perform the inversion
    let inv = uint256_inv_mod(a, modulus);

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

pub fn uint256_inv_mod(a: &[u64; 4], modulus: &[u64; 4]) -> Option<[u64; 4]> {
    let a_big = biguint_from_u64_digits(a);
    let modulus_big = biguint_from_u64_digits(modulus);
    let inv = a_big.modinv(&modulus_big);
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
    fn test_inv_mod_modulus_one() {
        let a = [0, 0, 0, 0];
        let modulus = [1, 0, 0, 0];
        let params = [a, modulus].concat();
        let mut results = [0; 5];
        fcall_uint256_inv_mod(&params, &mut results);
        let expected = [1, 0, 0, 0, 0];

        assert_eq!(results, expected);

        let a = [1, 0, 0, 0];
        let params = [a, modulus].concat();
        let mut results = [0; 5];
        fcall_uint256_inv_mod(&params, &mut results);
        let expected = [1, 0, 0, 0, 0];

        assert_eq!(results, expected);

        let a = [1, 2, 3, 4];
        let params = [a, modulus].concat();
        let mut results = [0; 5];
        fcall_uint256_inv_mod(&params, &mut results);
        let expected = [1, 0, 0, 0, 0];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_inv_mod_basic() {
        let a = [13, 0, 0, 0];
        let modulus = [12, 0, 0, 0];
        let params = [a, modulus].concat();
        let mut results = [0; 5];
        fcall_uint256_inv_mod(&params, &mut results);
        let expected = [1, 1, 0, 0, 0];

        assert_eq!(results, expected);

        let a = [6, 0, 0, 0];
        let params = [a, modulus].concat();
        let mut results = [0; 5];
        fcall_uint256_inv_mod(&params, &mut results);
        let expected = [0, 0, 0, 0, 0];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_inv_mod_rand() {
        let a = [0x48c964556ed2d279, 0xf692d9a779303069, 0xcc8d5e70e9f03415, 0xec53e64d5abb6d04];
        let modulus =
            [0xacca9ca1b4f3b763, 0x57d556242ac9c0ed, 0x6e3d795231a618cb, 0x36835e1b448f5df6];
        let params = [a, modulus].concat();
        let mut results = [0; 5];
        fcall_uint256_inv_mod(&params, &mut results);
        let expected =
            [1, 0xcede99fad6bbe0a2, 0x2c99e1d7ed681658, 0x2a8d1689b5e7bfaf, 0x20d97a86f6e5e3a4];

        assert_eq!(results, expected);

        let a = [0x844efa1db3aaaa7d, 0xfbc4783fdfea63b7, 0xd30100f0dc1f7df6, 0x444a];
        let params = [a, modulus].concat();
        let mut results = [0; 5];
        fcall_uint256_inv_mod(&params, &mut results);
        let expected = [0, 0, 0, 0, 0];

        assert_eq!(results, expected);
    }
}
