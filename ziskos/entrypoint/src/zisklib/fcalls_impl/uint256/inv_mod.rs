use num_bigint::BigUint;
use num_integer::Integer;

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};
use crate::zisklib::ModInvResult;

/// Compute `a^(-1) mod modulus`, returning 13 limbs: `[flag, w0..3, qa0..3, qm0..3]`.
///
/// If `flag` is 1 the inverse exists and is held in `w0..3` (the cofactor limbs are zero).
/// If `flag` is 0 no inverse exists and `w0..3` holds `gcd(a, modulus)`, `qa0..3 = a / gcd`,
/// `qm0..3 = modulus / gcd`, a witness proving `gcd(a, modulus) > 1`.
pub fn fcall_uint256_inv_mod(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a = &params[0..4].try_into().unwrap();
    let modulus = &params[4..8].try_into().unwrap();

    // Perform the inversion
    match uint256_inv_mod(a, modulus) {
        ModInvResult::Inverse(inv) => {
            results[0] = 1;
            results[1..5].copy_from_slice(&inv);
            results[5..13].fill(0);
        }
        ModInvResult::NoInverse { gcd, qa, qm } => {
            results[0] = 0;
            results[1..5].copy_from_slice(&gcd);
            results[5..9].copy_from_slice(&qa);
            results[9..13].copy_from_slice(&qm);
        }
    }

    13
}

pub fn uint256_inv_mod(a: &[u64; 4], modulus: &[u64; 4]) -> ModInvResult {
    let a_big = biguint_from_u64_digits(a);
    let modulus_big = biguint_from_u64_digits(modulus);

    if let Some(inv) = a_big.modinv(&modulus_big) {
        // The inverse exists: return it.
        ModInvResult::Inverse(n_u64_digits_from_biguint::<4>(&inv))
    } else {
        // No inverse exists, i.e. gcd(a, modulus) != 1. Return the gcd together with the cofactors
        // qa = a / gcd and qm = modulus / gcd as a witness. `modinv` panics on a zero modulus, so
        // here `modulus >= 1`, hence `gcd >= 1`; and `gcd != 1` (an inverse would exist otherwise),
        // so `gcd >= 2` and the divisions below never divide by zero.
        let gcd = a_big.gcd(&modulus_big);
        let qa = n_u64_digits_from_biguint::<4>(&(&a_big / &gcd));
        let qm = n_u64_digits_from_biguint::<4>(&(&modulus_big / &gcd));
        ModInvResult::NoInverse { gcd: n_u64_digits_from_biguint::<4>(&gcd), qa, qm }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_mod_modulus_one() {
        // Modulo 1 every residue is 0, so the inverse is 0.
        let modulus = [1, 0, 0, 0];
        for a in [[0, 0, 0, 0], [1, 0, 0, 0], [1, 2, 3, 4]] {
            let params = [a, modulus].concat();
            let mut results = [0; 13];
            let n = fcall_uint256_inv_mod(&params, &mut results);
            assert_eq!(n, 13);
            assert_eq!(results, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        }
    }

    #[test]
    fn test_inv_mod_basic() {
        let modulus = [12, 0, 0, 0];

        // 13 ≡ 1 (mod 12), so the inverse is 1.
        let a = [13, 0, 0, 0];
        let params = [a, modulus].concat();
        let mut results = [0; 13];
        fcall_uint256_inv_mod(&params, &mut results);
        assert_eq!(results, [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        // gcd(6, 12) = 6 > 1, so no inverse: witness gcd = 6, qa = 6/6 = 1, qm = 12/6 = 2.
        let a = [6, 0, 0, 0];
        let params = [a, modulus].concat();
        let mut results = [0; 13];
        fcall_uint256_inv_mod(&params, &mut results);
        assert_eq!(results, [0, 6, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
    }

    #[test]
    fn test_inv_mod_rand() {
        let modulus =
            [0xacca9ca1b4f3b763, 0x57d556242ac9c0ed, 0x6e3d795231a618cb, 0x36835e1b448f5df6];

        // Invertible: flag = 1, inverse in `w`, cofactor limbs zero.
        let a = [0x48c964556ed2d279, 0xf692d9a779303069, 0xcc8d5e70e9f03415, 0xec53e64d5abb6d04];
        let params = [a, modulus].concat();
        let mut results = [0; 13];
        fcall_uint256_inv_mod(&params, &mut results);
        assert_eq!(
            results,
            [
                1,
                0xcede99fad6bbe0a2,
                0x2c99e1d7ed681658,
                0x2a8d1689b5e7bfaf,
                0x20d97a86f6e5e3a4,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0
            ]
        );

        // Not invertible: here gcd(a, modulus) = a (a divides modulus), so qa = 1 and
        // qm = modulus / a.
        let a = [0x844efa1db3aaaa7d, 0xfbc4783fdfea63b7, 0xd30100f0dc1f7df6, 0x444a];
        let params = [a, modulus].concat();
        let mut results = [0; 13];
        fcall_uint256_inv_mod(&params, &mut results);
        assert_eq!(
            results,
            [
                0,
                0x844efa1db3aaaa7d,
                0xfbc4783fdfea63b7,
                0xd30100f0dc1f7df6,
                0x444a,
                1,
                0,
                0,
                0,
                0xcc58ffcfaf5f,
                0,
                0,
                0
            ]
        );
    }
}
