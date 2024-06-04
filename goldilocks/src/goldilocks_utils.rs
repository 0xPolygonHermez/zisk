extern crate gmp_mpfr_sys;
use gmp_mpfr_sys::gmp;
use p3_field::{AbstractField, PrimeField64};

use std::ffi::CString;
use std::mem::MaybeUninit;

use p3_goldilocks::Goldilocks;

pub trait DeserializeField {
    fn from_string(in1: &str, radix: i32) -> Self;
}

impl DeserializeField for p3_goldilocks::Goldilocks {
    fn from_string(in1: &str, radix: i32) -> p3_goldilocks::Goldilocks {
        unsafe {
            let mut mpz_value = {
                let mut mpz_value = MaybeUninit::uninit();
                gmp::mpz_init(mpz_value.as_mut_ptr());
                mpz_value.assume_init()
            };

            let mut mpz_prime = {
                let mut mpz_prime = MaybeUninit::uninit();
                gmp::mpz_init(mpz_prime.as_mut_ptr());
                mpz_prime.assume_init()
            };

            // Convert input string to CString
            let c_str = CString::new(in1).expect("CString::new failed");

            // Set aux from the input string with the given radix
            gmp::mpz_set_str(&mut mpz_value, c_str.as_ptr(), radix);

            // Define the Goldilocks prime as an mpz_t
            gmp::mpz_init_set_ui(&mut mpz_prime, Goldilocks::ORDER_U64);

            // Perform (aux + GOLDILOCKS_PRIME) % GOLDILOCKS_PRIME
            // gmp::mpz_add(&mut mpz_value, &mut mpz_value, &mpz_prime);
            gmp::mpz_mod(&mut mpz_value, &mut mpz_value, &mpz_prime);

            let result = Goldilocks::from_canonical_u64(gmp::mpz_get_ui(&mut mpz_value));

            gmp::mpz_clear(&mut mpz_value);
            gmp::mpz_clear(&mut mpz_prime);

            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Goldilocks, PrimeField64, DeserializeField};

    #[test]
    fn test_goldilocks() {
        let values: Vec<(String, u64)> = vec![
            ("-2".to_string(), Goldilocks::ORDER_U64 - 2),
            ("-1".to_string(), Goldilocks::ORDER_U64 - 1),
            ("0".to_string(), 0),
            ("1".to_string(), 1),
            ("2".to_string(), 2),
            ((Goldilocks::ORDER_U64 - 2).to_string(), Goldilocks::ORDER_U64 - 2),
            ((Goldilocks::ORDER_U64 - 1).to_string(), Goldilocks::ORDER_U64 - 1),
            ((Goldilocks::ORDER_U64).to_string(), 0),
            ((Goldilocks::ORDER_U64 + 1).to_string(), 1),
            ((Goldilocks::ORDER_U64 + 2).to_string(), 2),
            ((2 * Goldilocks::ORDER_U64 as u128 - 2).to_string(), Goldilocks::ORDER_U64 - 2),
            ((2 * Goldilocks::ORDER_U64 as u128 - 1).to_string(), Goldilocks::ORDER_U64 - 1),
            ((2 * Goldilocks::ORDER_U64 as u128).to_string(), 0),
            ((2 * Goldilocks::ORDER_U64 as u128 + 1).to_string(), 1),
            ((2 * Goldilocks::ORDER_U64 as u128 + 2).to_string(), 2),
        ];

        for val in values {
            assert_eq!(Goldilocks::from_string(&val.0, 10).as_canonical_u64(), val.1 % Goldilocks::ORDER_U64);
        }
    }
}
