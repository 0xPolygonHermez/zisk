cfg_if::cfg_if! {
    if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
        use lib_c::secp256k1_fp_parity_sqrt_c;

        pub fn fcall_secp256k1_fp_sqrt(params: &[u64], results: &mut [u64]) -> i64 {
            let input = &params[0..4];
            let parity = params[4];

            // Perform the square root
            let res_c_call = secp256k1_fp_parity_sqrt_c(input, parity, results);
            if res_c_call == 0 {
                5
            } else {
                res_c_call as i64
            }
        }
    } else {
        use lazy_static::lazy_static;
        use num_bigint::BigUint;
        use num_traits::{One, Zero};

        use super::utils::{from_limbs_le, to_limbs_le};

        lazy_static! {
            pub static ref P: BigUint = BigUint::parse_bytes(
                b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
                16
            )
            .unwrap();

            pub static ref P_HALF: BigUint = BigUint::parse_bytes(
                b"7fffffffffffffffffffffffffffffffffffffffffffffffffffffff7ffffe17",
                16
            )
            .unwrap();

            pub static ref P_DIV_4: BigUint = BigUint::parse_bytes(
                b"3fffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffff0c",
                16
            )
            .unwrap();
        }

        pub fn fcall_secp256k1_fp_sqrt(params: &[u64], results: &mut [u64]) -> i64 {
            // Get the input
            let a: &[u64; 4] = &params[0..4].try_into().unwrap();
            let parity = params[4];

            // Perform the square root
            secp256k1_fp_sqrt(a, parity, results);

            5
        }

        fn secp256k1_fp_sqrt(a: &[u64; 4], parity: u64, results: &mut [u64]) {
            let a_big = from_limbs_le(a);
            if a_big.is_zero() {
                // If a is zero, the square root is also zero
                results[0] = 1; // Indicate that a solution exists
                return;
            }

            // Check if a is a quadratic residue
            let legendre = a_big.modpow(&P_HALF, &P);
            if legendre != BigUint::one() {
                // If the Legendre symbol is not 1, there is no square root
                results[0] = 0; // Indicate that no solution exists
                return;
            }

            results[0] = 1; // Indicate that a solution exists

            // Calculate the square root
            let sqrt = a_big.modpow(&P_DIV_4, &P);

            // Adjust the result based on the parity
            let mut limbs = to_limbs_le(&sqrt);
            // If parities does not coincide, we need to take the negative square root
            if (&sqrt & BigUint::one()) != BigUint::from(parity) {
                let neg_sqrt = P.clone() - &sqrt;
                limbs = to_limbs_le(&neg_sqrt);
            }

            results[1..5].copy_from_slice(&limbs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt_one() {
        let x = [1, 0, 0, 0];
        let parity = 1u64;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt = [1, 0, 0, 0];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sol = results[0];
        assert!(has_sol == 1);
        assert_eq!(results[1..5], expected_sqrt);

        let parity = 0u64;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt =
            [0xfffffffefffffc2e, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sol = results[0];
        assert!(has_sol == 1);
        assert_eq!(results[1..5], expected_sqrt);
    }

    #[test]
    fn test_sqrt() {
        let x = [0x643764b2faa1592a, 0x4ac3ab52286f702a, 0x6591d88c833ffd4f, 0xc6fb7a1e514eac26];
        let parity = 0u64;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt =
            [0xa3d2fb0160f29df6, 0x3ebce4d565b52649, 0x4cdec0bf5c968639, 0x123e42087c415355];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sol = results[0];
        assert!(has_sol == 1);
        assert_eq!(results[1..5], expected_sqrt);

        let parity = 1u64;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt =
            [0x5c2d04fd9f0d5e39, 0xc1431b2a9a4ad9b6, 0xb3213f40a36979c6, 0xedc1bdf783beacaa];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sol = results[0];
        assert!(has_sol == 1);
        assert_eq!(results[1..5], expected_sqrt);
    }

    #[test]
    fn test_no_sqrt() {
        let x = [0x643764b2faa1592c, 0x4ac3ab52286f702a, 0x6591d88c833ffd4f, 0xc6fb7a1e514eac26];
        let parity = 0u64;
        let params = [x[0], x[1], x[2], x[3], parity];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sol = results[0];
        assert!(has_sol == 0);
    }
}
